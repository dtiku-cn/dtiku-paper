# 实时流量监控后台页面

## 功能概述

基于 Arroyo 流处理引擎产生的实时统计数据，在管理后台新增了一个可视化监控页面，用于实时查看网站流量、安全防护、用户行为等关键指标。

## 实现内容

### 1. 后端 API (`dtiku-backend/src/router/stats.rs`)

提供 5 个 RESTful API 端点，从 Redis 读取 Arroyo 流处理产生的实时数据：

- **GET `/api/stats/blocked-ips`** - 获取 DDoS 防护封禁的 IP 列表
- **GET `/api/stats/suspicious-users`** - 获取异常用户行为监控列表
- **GET `/api/stats/traffic`** - 获取实时流量统计数据
- **GET `/api/stats/rate-limits`** - 获取智能限流配置
- **GET `/api/stats/hot-urls`** - 获取热门 URL 访问排行

所有 API 使用 Redis SCAN 命令进行安全的键遍历，避免阻塞 Redis。

### 2. 前端监控页面 (`dtiku-backend/frontend/src/pages/RealtimeStats.tsx`)

完整的实时监控仪表盘，包含：

#### 统计卡片
- 总请求数
- 封禁 IP 数量
- 异常用户数量
- 热门 URL 数量

#### 可视化图表
- **状态码分布饼图** - 展示 2xx/3xx/4xx/5xx 的请求占比
- **Top 10 QPS 端点柱状图** - 显示 QPS 最高的 API 端点
- **Top 10 热门 URL 柱状图** - 显示访问量最高的 URL

#### 详细数据表格
1. **DDoS 防护表** - 显示被封禁的 IP、请求次数、封禁时间
2. **异常用户监控表** - 显示用户 ID、请求次数、错误率、风险等级
3. **智能限流配置表** - 显示各端点的当前 QPS、错误率、建议限流值
4. **热门 URL 分析表** - 显示 URL 路径、请求次数、4xx/5xx 错误统计

#### 响应式设计
- 支持移动端、平板、桌面端不同屏幕尺寸
- 表格列根据屏幕大小自适应显示/隐藏
- 图表尺寸和字体自动调整

#### 自动刷新
- 页面每 30 秒自动刷新数据
- 支持手动点击"刷新数据"按钮

### 3. 类型定义 (`dtiku-backend/frontend/src/types.ts`)

新增 TypeScript 接口定义：
- `BlockedIp` - 封禁 IP 数据结构
- `SuspiciousUser` - 异常用户数据结构
- `TrafficStats` - 流量统计数据结构
- `RateLimitConfig` - 限流配置数据结构
- `HotUrl` - 热门 URL 数据结构

### 4. API 服务层 (`dtiku-backend/frontend/src/services/api.ts`)

新增 `RealtimeStatsService` 服务，封装所有实时统计相关的 API 调用。

### 5. 路由配置 (`dtiku-backend/frontend/src/App.tsx`)

- 在左侧菜单添加"实时监控"菜单项（仪表盘图标）
- 注册路由 `/realtime-stats` 指向监控页面

## 使用方法

### 启动服务

1. **启动 Arroyo 流处理引擎**（参考 `sql/arroyo/README.md`）
   ```bash
   # 确保 Arroyo 已配置并运行流处理任务
   ```

2. **启动后端服务**
   ```bash
   cd dtiku-backend
   cargo run
   ```

3. **访问管理后台**
   ```
   打开浏览器访问: http://localhost:8080
   点击左侧菜单"实时监控"
   ```

### 数据流向

```
Nginx 访问日志 
  → Vector (采集) 
    → Arroyo (流处理) 
      → Redis (存储实时统计) 
        → 后端 API (读取) 
          → 前端页面 (展示)
```

## 数据说明

### Redis 键命名规则

根据 `sql/arroyo/arroyo.sql` 中的定义：

- `block_ip:{ip}` - 封禁 IP 哈希表
- `suspicious_user:{user_id}` - 异常用户哈希表
- `traffic:stats:{metric_key}` - 流量统计哈希表
- `rate_limit:{endpoint}` - 限流配置哈希表
- `hot_url:{url_path}` - 热门 URL 哈希表

### 数据过期策略

- **封禁 IP**: 默认封禁 1 小时（可在 Arroyo SQL 中调整）
- **爬虫/机器人**: 默认封禁 2 小时
- **其他数据**: 由 Arroyo 窗口时间决定，定期更新覆盖

## 监控指标说明

### DDoS 防护
- **触发条件**: 1分钟内请求 > 100次 或 10秒内请求 > 30次
- **防护措施**: 自动封禁 IP 一段时间

### 账户安全
- **风险等级**:
  - **高风险**: 请求数 > 500 或 错误率 > 50%
  - **中风险**: 请求数 > 200 或 错误率 > 30%
  - **低风险**: 其他情况

### 智能限流
- **算法**: 根据错误率动态调整限流值
  - 错误率 > 10%: 限流至当前 QPS 的 50%
  - 错误率 > 5%: 限流至当前 QPS 的 70%
  - 正常情况: 允许当前 QPS 的 150%

### 爬虫检测
- 检测规则: User-Agent 包含 bot/crawler/spider/scraper 关键词
- 白名单: Googlebot、Bingbot、Baidu、Sogou（合法搜索引擎）
- 触发封禁: 5分钟内请求 > 50次

## 扩展建议

1. **告警功能**: 集成邮件/钉钉/企业微信告警，当异常用户或攻击达到阈值时自动通知
2. **历史趋势**: 将实时数据定期持久化到 PostgreSQL，支持历史趋势分析
3. **自定义规则**: 支持在后台页面动态配置封禁规则和限流阈值
4. **导出功能**: 支持导出统计数据为 CSV/Excel
5. **地域分析**: 整合 IP 地理位置库，展示攻击来源地图
