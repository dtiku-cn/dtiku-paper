use anyhow::Context;
use serde::{Deserialize, Serialize};
use spring_redis::Redis;
use spring_web::{
    axum::{response::IntoResponse, Json},
    error::Result,
    extractor::{Component, Query},
    get,
};
use std::collections::HashSet;

/// 使用 SCAN 命令扫描匹配 pattern 的所有 key（非阻塞）
async fn scan_keys(redis: &mut Redis, pattern: &str) -> anyhow::Result<Vec<String>> {
    let mut keys = Vec::new();
    let mut cursor: u64 = 0;
    
    loop {
        // SCAN cursor MATCH pattern COUNT 100
        let result: (u64, Vec<String>) = spring_redis::redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(pattern)
            .arg("COUNT")
            .arg(100)
            .query_async(redis)
            .await
            .context("Redis SCAN failed")?;
        
        cursor = result.0;
        keys.extend(result.1);
        
        // cursor 为 0 表示扫描完成
        if cursor == 0 {
            break;
        }
    }
    
    Ok(keys)
}

/// 批量获取多个 key 的值（使用 MGET 命令）
async fn mget_values(redis: &mut Redis, keys: &[String]) -> anyhow::Result<Vec<Option<String>>> {
    if keys.is_empty() {
        return Ok(Vec::new());
    }
    
    // MGET key1 key2 key3 ...
    let mut cmd = spring_redis::redis::cmd("MGET");
    for key in keys {
        cmd.arg(key);
    }
    
    let values: Vec<Option<String>> = cmd
        .query_async(redis)
        .await
        .context("Redis MGET failed")?;
    
    Ok(values)
}

// ============ 请求参数 ============

#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    pub host: Option<String>,
}

// ============ 响应结构体 ============

/// DDoS 防护：封禁 IP 列表
#[derive(Debug, Serialize, Deserialize)]
pub struct BlockedIp {
    pub host: String,
    pub ip: String,
    pub request_count: i64,
    pub first_seen: String,
    pub last_seen: String,
    pub block_until: String,
}

/// 账户安全：异常用户
#[derive(Debug, Serialize, Deserialize)]
pub struct SuspiciousUser {
    pub host: String,
    pub user_id: String,
    pub user_name: String,
    pub request_count: i64,
    pub error_rate: f64,
    pub window_start: String,
    pub window_end: String,
    pub risk_level: String,
}

/// 流量统计
#[derive(Debug, Serialize, Deserialize)]
pub struct TrafficStats {
    pub host: String,
    pub metric_type: String,
    pub metric_key: String,
    pub value: i64,
    pub window_start: String,
    pub window_end: String,
}

/// 限流配置
#[derive(Debug, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub host: String,
    pub endpoint: String,
    pub current_qps: i64,
    pub avg_response_time: f64,
    pub error_rate: f64,
    pub suggested_limit: i64,
    pub window_time: String,
}

/// URL 热点
#[derive(Debug, Serialize, Deserialize)]
pub struct HotUrl {
    pub host: String,
    pub url_path: String,
    pub request_count: i64,
    pub avg_response_size: f64,
    pub status_4xx_count: i64,
    pub status_5xx_count: i64,
    pub window_start: String,
    pub window_end: String,
}

// ============ API 处理器 ============

/// 获取所有 hosts 列表
#[get("/api/stats/hosts")]
async fn get_hosts(Component(mut redis): Component<Redis>) -> Result<impl IntoResponse> {
    let mut hosts = HashSet::new();
    
    // 扫描所有相关的 Redis key 获取 host 列表
    // 新 key 格式：traffic:{type}:{host}:{field}
    let prefixes = vec!["traffic:block_ip:", "traffic:suspicious_user:", "traffic:stats:", "traffic:rate_limit:", "traffic:hot_url:"];
    
    for prefix in prefixes {
        let pattern = format!("{}*", prefix);
        if let Ok(keys) = scan_keys(&mut redis, &pattern).await {
            for key in keys {
                // 提取 host 部分（key 格式：traffic:{type}:{host}:{field}）
                if let Some(remainder) = key.strip_prefix(prefix) {
                    // 查找第一个冒号，之前的部分就是 host
                    if let Some(colon_pos) = remainder.find(':') {
                        let host = &remainder[..colon_pos];
                        if !host.is_empty() {
                            hosts.insert(host.to_string());
                        }
                    }
                }
            }
        }
    }
    
    let mut host_list: Vec<String> = hosts.into_iter().collect();
    host_list.sort();
    
    Ok(Json(host_list))
}

/// 获取封禁 IP 列表
#[get("/api/stats/blocked-ips")]
async fn get_blocked_ips(
    Query(query): Query<StatsQuery>,
    Component(mut redis): Component<Redis>,
) -> Result<impl IntoResponse> {
    let host = query.host.as_deref().unwrap_or("");
    let pattern = format!("traffic:block_ip:{}:*", host);
    
    let keys = scan_keys(&mut redis, &pattern)
        .await
        .context("Redis SCAN block_ip failed")?;

    if keys.is_empty() {
        return Ok(Json(Vec::new()));
    }

    // 批量获取所有 key 的值
    let values = mget_values(&mut redis, &keys)
        .await
        .context("Redis MGET block_ip failed")?;

    let mut blocked_ips = Vec::new();
    
    // 处理批量获取的结果
    for (key, json_str_opt) in keys.iter().zip(values.iter()) {
        // 从 key 中提取 IP：traffic:block_ip:{host}:{ip}
        if let Some(ip) = key.strip_prefix(&format!("traffic:block_ip:{}:", host)) {
            if let Some(json_str) = json_str_opt {
                if let Ok(mut item) = serde_json::from_str::<BlockedIp>(json_str) {
                    item.host = host.to_string();
                    item.ip = ip.to_string();
                    blocked_ips.push(item);
                }
            }
        }
    }

    // 按请求数排序
    blocked_ips.sort_by(|a, b| b.request_count.cmp(&a.request_count));

    Ok(Json(blocked_ips))
}

/// 获取异常用户列表
#[get("/api/stats/suspicious-users")]
async fn get_suspicious_users(
    Query(query): Query<StatsQuery>,
    Component(mut redis): Component<Redis>,
) -> Result<impl IntoResponse> {
    let host = query.host.as_deref().unwrap_or("");
    let pattern = format!("traffic:suspicious_user:{}:*", host);
    
    let keys = scan_keys(&mut redis, &pattern)
        .await
        .context("Redis SCAN suspicious_user failed")?;

    if keys.is_empty() {
        return Ok(Json(Vec::new()));
    }

    // 批量获取所有 key 的值
    let values = mget_values(&mut redis, &keys)
        .await
        .context("Redis MGET suspicious_user failed")?;

    let mut users = Vec::new();
    
    // 处理批量获取的结果
    for (key, json_str_opt) in keys.iter().zip(values.iter()) {
        // 从 key 中提取 user_id：traffic:suspicious_user:{host}:{user_id}
        if let Some(user_id) = key.strip_prefix(&format!("traffic:suspicious_user:{}:", host)) {
            if let Some(json_str) = json_str_opt {
                if let Ok(mut user) = serde_json::from_str::<SuspiciousUser>(json_str) {
                    user.host = host.to_string();
                    user.user_id = user_id.to_string();
                    users.push(user);
                }
            }
        }
    }

    // 按风险等级和请求数排序
    users.sort_by(|a, b| {
        let risk_order = |level: &str| match level {
            "high" => 0,
            "medium" => 1,
            _ => 2,
        };
        risk_order(&a.risk_level)
            .cmp(&risk_order(&b.risk_level))
            .then(b.request_count.cmp(&a.request_count))
    });

    Ok(Json(users))
}

/// 获取流量统计
#[get("/api/stats/traffic")]
async fn get_traffic_stats(
    Query(query): Query<StatsQuery>,
    Component(mut redis): Component<Redis>,
) -> Result<impl IntoResponse> {
    let host = query.host.as_deref().unwrap_or("");
    let pattern = format!("traffic:stats:{}:*", host);
    
    let keys = scan_keys(&mut redis, &pattern)
        .await
        .context("Redis SCAN traffic failed")?;

    if keys.is_empty() {
        return Ok(Json(Vec::new()));
    }

    // 批量获取所有 key 的值
    let values = mget_values(&mut redis, &keys)
        .await
        .context("Redis MGET traffic failed")?;

    let mut stats = Vec::new();
    
    // 处理批量获取的结果
    for (key, json_str_opt) in keys.iter().zip(values.iter()) {
        // 从 key 中提取 metric_type 和 metric_key：traffic:stats:{host}:{metric_type}:{metric_key}
        if let Some(remainder) = key.strip_prefix(&format!("traffic:stats:{}:", host)) {
            // 找到最后一个冒号分隔符
            if let Some(last_colon) = remainder.rfind(':') {
                let metric_type = &remainder[..last_colon];
                let metric_key = &remainder[last_colon + 1..];
                
                if let Some(json_str) = json_str_opt {
                    if let Ok(mut stat) = serde_json::from_str::<TrafficStats>(json_str) {
                        stat.host = host.to_string();
                        stat.metric_type = metric_type.to_string();
                        stat.metric_key = metric_key.to_string();
                        stats.push(stat);
                    }
                }
            }
        }
    }

    Ok(Json(stats))
}

/// 获取限流配置
#[get("/api/stats/rate-limits")]
async fn get_rate_limits(
    Query(query): Query<StatsQuery>,
    Component(mut redis): Component<Redis>,
) -> Result<impl IntoResponse> {
    let host = query.host.as_deref().unwrap_or("");
    let pattern = format!("traffic:rate_limit:{}:*", host);
    
    let keys = scan_keys(&mut redis, &pattern)
        .await
        .context("Redis SCAN rate_limit failed")?;

    if keys.is_empty() {
        return Ok(Json(Vec::new()));
    }

    // 批量获取所有 key 的值
    let values = mget_values(&mut redis, &keys)
        .await
        .context("Redis MGET rate_limit failed")?;

    let mut configs = Vec::new();
    
    // 处理批量获取的结果
    for (key, json_str_opt) in keys.iter().zip(values.iter()) {
        // 从 key 中提取 endpoint：traffic:rate_limit:{host}:{endpoint}
        if let Some(endpoint) = key.strip_prefix(&format!("traffic:rate_limit:{}:", host)) {
            if let Some(json_str) = json_str_opt {
                if let Ok(mut config) = serde_json::from_str::<RateLimitConfig>(json_str) {
                    config.host = host.to_string();
                    config.endpoint = endpoint.to_string();
                    configs.push(config);
                }
            }
        }
    }

    // 按 QPS 排序
    configs.sort_by(|a, b| b.current_qps.cmp(&a.current_qps));

    Ok(Json(configs))
}

/// 获取热门 URL
#[get("/api/stats/hot-urls")]
async fn get_hot_urls(
    Query(query): Query<StatsQuery>,
    Component(mut redis): Component<Redis>,
) -> Result<impl IntoResponse> {
    let host = query.host.as_deref().unwrap_or("");
    let pattern = format!("traffic:hot_url:{}:*", host);
    
    let keys = scan_keys(&mut redis, &pattern)
        .await
        .context("Redis SCAN hot_url failed")?;

    if keys.is_empty() {
        return Ok(Json(Vec::new()));
    }

    // 批量获取所有 key 的值
    let values = mget_values(&mut redis, &keys)
        .await
        .context("Redis MGET hot_url failed")?;

    let mut urls = Vec::new();
    
    // 处理批量获取的结果
    for (key, json_str_opt) in keys.iter().zip(values.iter()) {
        // 从 key 中提取 url_path：traffic:hot_url:{host}:{url_path}
        if let Some(url_path) = key.strip_prefix(&format!("traffic:hot_url:{}:", host)) {
            if let Some(json_str) = json_str_opt {
                if let Ok(mut url) = serde_json::from_str::<HotUrl>(json_str) {
                    url.host = host.to_string();
                    url.url_path = url_path.to_string();
                    urls.push(url);
                }
            }
        }
    }

    // 按请求数排序，取 Top 50
    urls.sort_by(|a, b| b.request_count.cmp(&a.request_count));
    urls.truncate(50);

    Ok(Json(urls))
}

