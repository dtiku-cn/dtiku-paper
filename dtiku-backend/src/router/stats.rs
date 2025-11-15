use anyhow::Context;
use serde::{Deserialize, Serialize};
use spring_redis::Redis;
use spring_web::{
    axum::{response::IntoResponse, Json},
    error::Result,
    extractor::Component,
    get,
};

// ============ 响应结构体 ============

/// DDoS 防护：封禁 IP 列表
#[derive(Debug, Serialize, Deserialize)]
pub struct BlockedIp {
    pub ip: String,
    pub request_count: i64,
    pub first_seen: String,
    pub last_seen: String,
    pub block_until: String,
}

/// 账户安全：异常用户
#[derive(Debug, Serialize, Deserialize)]
pub struct SuspiciousUser {
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
    pub metric_type: String,
    pub metric_key: String,
    pub value: i64,
    pub window_start: String,
    pub window_end: String,
}

/// 限流配置
#[derive(Debug, Serialize, Deserialize)]
pub struct RateLimitConfig {
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
    pub url_path: String,
    pub request_count: i64,
    pub avg_response_size: f64,
    pub status_4xx_count: i64,
    pub status_5xx_count: i64,
    pub window_start: String,
    pub window_end: String,
}

// ============ API 处理器 ============

/// 获取封禁 IP 列表
#[get("/api/stats/blocked-ips")]
async fn get_blocked_ips(Component(mut redis): Component<Redis>) -> Result<impl IntoResponse> {
    // 直接从固定的 hash key 获取所有数据
    let data: Vec<(String, String)> = spring_redis::redis::cmd("HGETALL")
        .arg("block_ip:")
        .query_async(&mut redis)
        .await
        .context("Redis HGETALL block_ip failed")?;

    let mut blocked_ips = Vec::new();
    
    // 将扁平的 (field, value) 转换为结构体
    // 每个 IP 是一个 hash field，value 是 JSON 字符串
    for (ip, json_str) in data {
        if let Ok(mut item) = serde_json::from_str::<BlockedIp>(&json_str) {
            item.ip = ip; // 确保 IP 字段正确
            blocked_ips.push(item);
        }
    }

    // 按请求数排序
    blocked_ips.sort_by(|a, b| b.request_count.cmp(&a.request_count));

    Ok(Json(blocked_ips))
}

/// 获取异常用户列表
#[get("/api/stats/suspicious-users")]
async fn get_suspicious_users(
    Component(mut redis): Component<Redis>,
) -> Result<impl IntoResponse> {
    let data: Vec<(String, String)> = spring_redis::redis::cmd("HGETALL")
        .arg("suspicious_user:")
        .query_async(&mut redis)
        .await
        .context("Redis HGETALL suspicious_user failed")?;

    let mut users = Vec::new();
    
    for (user_id, json_str) in data {
        if let Ok(mut user) = serde_json::from_str::<SuspiciousUser>(&json_str) {
            user.user_id = user_id;
            users.push(user);
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
async fn get_traffic_stats(Component(mut redis): Component<Redis>) -> Result<impl IntoResponse> {
    let data: Vec<(String, String)> = spring_redis::redis::cmd("HGETALL")
        .arg("traffic:stats:")
        .query_async(&mut redis)
        .await
        .context("Redis HGETALL traffic failed")?;

    let mut stats = Vec::new();
    
    for (metric_key, json_str) in data {
        if let Ok(mut stat) = serde_json::from_str::<TrafficStats>(&json_str) {
            stat.metric_key = metric_key;
            stats.push(stat);
        }
    }

    Ok(Json(stats))
}

/// 获取限流配置
#[get("/api/stats/rate-limits")]
async fn get_rate_limits(Component(mut redis): Component<Redis>) -> Result<impl IntoResponse> {
    let data: Vec<(String, String)> = spring_redis::redis::cmd("HGETALL")
        .arg("rate_limit:")
        .query_async(&mut redis)
        .await
        .context("Redis HGETALL rate_limit failed")?;

    let mut configs = Vec::new();
    
    for (endpoint, json_str) in data {
        if let Ok(mut config) = serde_json::from_str::<RateLimitConfig>(&json_str) {
            config.endpoint = endpoint;
            configs.push(config);
        }
    }

    // 按 QPS 排序
    configs.sort_by(|a, b| b.current_qps.cmp(&a.current_qps));

    Ok(Json(configs))
}

/// 获取热门 URL
#[get("/api/stats/hot-urls")]
async fn get_hot_urls(Component(mut redis): Component<Redis>) -> Result<impl IntoResponse> {
    let data: Vec<(String, String)> = spring_redis::redis::cmd("HGETALL")
        .arg("hot_url:")
        .query_async(&mut redis)
        .await
        .context("Redis HGETALL hot_url failed")?;

    let mut urls = Vec::new();
    
    for (url_path, json_str) in data {
        if let Ok(mut url) = serde_json::from_str::<HotUrl>(&json_str) {
            url.url_path = url_path;
            urls.push(url);
        }
    }

    // 按请求数排序，取 Top 50
    urls.sort_by(|a, b| b.request_count.cmp(&a.request_count));
    urls.truncate(50);

    Ok(Json(urls))
}

