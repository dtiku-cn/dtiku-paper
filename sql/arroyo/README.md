# Arroyo 流处理配置指南

## 📊 技术栈总览

```
┌─────────────────────────────────────────────────────────────────┐
│                        Arroyo 流处理系统                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Nginx Logs  ──→  Vector  ──→  Arroyo  ──→  Redis             │
│                     ↓              ↓           ↓                │
│                  Parse        Process      Storage              │
│                                                                 │
│  监控层: Prometheus + Grafana                                    │
│  存储层: PostgreSQL (Arroyo 元数据) + Redis (流处理结果)          │
│  编排层: Docker Compose                                          │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 核心组件

- **Arroyo**: Rust 流处理引擎（替代 Flink）
- **Vector**: 高性能日志采集器
- **Redis**: 流处理结果存储和缓存
- **PostgreSQL**: Arroyo 元数据存储

## 🎯 功能概览

本 Arroyo SQL 配置实现了完整的**实时访问监控和安全防护系统**，涵盖四大核心功能：

### 1. 实时 DDoS 防护 ✅
- **高频 IP 检测**：1分钟内超过100次或10秒内超过30次请求自动封禁
- **爬虫/机器人识别**：基于 User-Agent 特征 + 访问频率检测恶意爬虫
- **自动封禁**：检测到的异常IP写入 Redis `block_ip:*`，自动过期（1-2小时）

### 2. 账户安全监控 ✅
- **异常行为检测**：监控已登录用户的高频请求和高错误率
- **风险等级分级**：low/medium/high 三级风险评估
- **实时告警数据**：写入 Redis `suspicious_user:*`，保留30分钟

### 3. 流量分析仪表盘 ✅
- **总体流量统计**：每分钟请求数、状态码分布
- **Host 维度统计**：多租户场景下的分域名流量
- **URL 热点分析**：Top 热门访问路径（自动聚合 `/paper/123` → `/paper/:id`）
- **实时指标**：所有数据写入 Redis `traffic:stats:*`，5分钟 TTL

### 4. 智能限流 ✅
- **动态 QPS 计算**：实时计算每个 API 端点的 QPS
- **自适应限流**：根据错误率自动调整限流阈值
  - 错误率 > 10%：限流至 50% QPS
  - 错误率 > 5%：限流至 70% QPS
  - 正常情况：允许 150% QPS
- **配置输出**：写入 Redis `rate_limit:*`，每分钟更新

---

## 🔗 Rust 集成示例

### 1. 读取封禁 IP 列表（DDoS 防护）
```rust
// dtiku-web/src/middleware/ip_blocker.rs
use spring_redis::RedisService;
use axum::{extract::State, middleware::Next, response::Response};

pub async fn ip_blocker_middleware(
    State(redis): State<RedisService>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let client_ip = request
        .headers()
        .get("X-Real-IP")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown");

    // 检查 IP 是否在封禁列表中
    let key = format!("block_ip:{}", client_ip);
    if redis.exists(&key).await? {
        let info: BlockInfo = redis.hgetall(&key).await?;
        tracing::warn!("Blocked IP {} - Count: {}", client_ip, info.request_count);
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(request).await)
}

#[derive(Deserialize)]
struct BlockInfo {
    request_count: i64,
    block_until: String,
}
```

### 2. 智能限流中间件
```rust
// dtiku-web/src/middleware/rate_limiter.rs
use spring_redis::RedisService;

pub async fn dynamic_rate_limiter(
    State(redis): State<RedisService>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = request.uri().path();
    
    // 从 Redis 获取动态限流配置
    let config_key = format!("rate_limit:{}", path);
    let config: Option<RateLimitConfig> = redis.hgetall(&config_key).await.ok();
    
    let limit = config
        .map(|c| c.suggested_limit)
        .unwrap_or(100);  // 默认限流100 QPS
    
    // 使用 Token Bucket 算法检查限流
    let counter_key = format!("req_count:{}:{}", path, get_current_minute());
    let count: i64 = redis.incr(&counter_key, 1).await?;
    redis.expire(&counter_key, 60).await?;
    
    if count > limit {
        tracing::warn!("Rate limit exceeded for {} - {}/{}", path, count, limit);
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    
    Ok(next.run(request).await)
}

#[derive(Deserialize)]
struct RateLimitConfig {
    suggested_limit: i64,
    current_qps: i64,
    error_rate: f64,
}
```

### 3. 流量仪表盘 API
```rust
// dtiku-backend/src/router/dashboard.rs
use spring_web::get;

#[get("/api/dashboard/traffic")]
async fn get_traffic_stats(
    redis: State<RedisService>,
) -> Result<Json<TrafficStats>, StatusCode> {
    // 读取总请求数
    let total: HashMap<String, String> = redis
        .hgetall("traffic:stats:total_requests")
        .await?;
    
    // 读取状态码分布
    let status_2xx = redis.hgetall("traffic:stats:by_status:2xx").await?;
    let status_4xx = redis.hgetall("traffic:stats:by_status:4xx").await?;
    let status_5xx = redis.hgetall("traffic:stats:by_status:5xx").await?;
    
    // 读取热门URL
    let hot_urls: Vec<(String, f64)> = redis
        .zrevrange_withscores("hot_urls", 0, 9)  // Top 10
        .await?;
    
    Ok(Json(TrafficStats {
        total_requests: total.get("value").and_then(|v| v.parse().ok()).unwrap_or(0),
        status_distribution: StatusDistribution {
            success: status_2xx.get("value").and_then(|v| v.parse().ok()).unwrap_or(0),
            client_error: status_4xx.get("value").and_then(|v| v.parse().ok()).unwrap_or(0),
            server_error: status_5xx.get("value").and_then(|v| v.parse().ok()).unwrap_or(0),
        },
        hot_urls: hot_urls.into_iter().map(|(url, count)| HotUrl {
            path: url,
            count: count as i64,
        }).collect(),
    }))
}
```

### 4. 异常用户告警
```rust
// dtiku-base/src/service/security_monitor.rs
use spring_job::cron;

#[cron("0 */5 * * * *")]  // 每5分钟检查一次
async fn check_suspicious_users(redis: RedisService) -> Result<()> {
    let keys: Vec<String> = redis.keys("suspicious_user:*").await?;
    
    for key in keys {
        let user: SuspiciousUser = redis.hgetall(&key).await?;
        
        if user.risk_level == "high" {
            // 发送告警通知
            tracing::error!(
                "High risk user detected: {} ({}), requests: {}, error_rate: {:.2}%",
                user.user_name,
                user.user_id,
                user.request_count,
                user.error_rate * 100.0
            );
            
            // 可选：自动禁用账户
            if user.error_rate > 0.8 {
                // suspend_user(&user.user_id).await?;
            }
        }
    }
    
    Ok(())
}
```

---

## ⚙️ 调优建议

### 1. 阈值调整
根据实际流量调整各任务的阈值：

```sql
-- 任务1：DDoS 防护阈值
HAVING COUNT(*) > 100  -- 调整为适合您的流量水平

-- 任务2：账户安全阈值
HAVING COUNT(*) > 100  -- 高频请求阈值
    OR ... > 0.2       -- 错误率阈值

-- 任务4：限流计算窗口
GROUP BY TUMBLE(INTERVAL '1 minute')  -- 可调整为 30 秒
```

### 2. 窗口大小优化
- **滑动窗口 (HOP)**：适用于需要平滑检测的场景（DDoS）
- **滚动窗口 (TUMBLE)**：适用于固定周期统计（流量报表）
- **会话窗口**：未使用，可用于检测用户会话异常

### 3. Redis 内存管理
```bash
# 设置 Redis 最大内存和淘汰策略
redis-cli CONFIG SET maxmemory 2gb
redis-cli CONFIG SET maxmemory-policy allkeys-lru
```

### 4. Arroyo 性能优化
```yaml
# Arroyo 配置
arroyo:
  environment:
    - CHECKPOINT_INTERVAL=60s  # checkpoint 间隔
    - PARALLELISM=4            # 并行度
    - MAX_PARALLEL_CHECKPOINTS=2
```

## 📝 扩展建议

### 1. 地理位置分析
```sql
-- 需要集成 GeoIP 库
CREATE TABLE geo_stats AS
SELECT 
    country,
    COUNT(*) as requests
FROM nginx_access_log
    JOIN geoip(remote_addr) as geo
GROUP BY country, TUMBLE(INTERVAL '5 minutes');
```

### 2. 实时告警集成
```sql
-- 通过 Webhook Sink 发送告警
CREATE TABLE alert_webhook (
    message TEXT,
    severity TEXT
) WITH (
    connector = 'webhook',
    endpoint = 'https://your-alert-system.com/api/alerts'
);
```

## 📚 参考资料

- [Arroyo 官方文档](https://doc.arroyo.dev/)
- [Vector 配置指南](https://vector.dev/docs/)
- [Redis Streams](https://redis.io/docs/data-types/streams/)
- [Nginx 日志配置](https://nginx.org/en/docs/http/ngx_http_log_module.html)