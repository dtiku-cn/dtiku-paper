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
    let mut blocked_ips = Vec::new();
    let mut cursor = 0u64;

    // 使用 SCAN 遍历所有 block_ip:* 的 hash key
    loop {
        let (new_cursor, keys): (u64, Vec<String>) = spring_redis::redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg("block_ip:*")
            .arg("COUNT")
            .arg(100)
            .query_async(&mut redis)
            .await
            .context("Redis SCAN block_ip failed")?;

        for key in keys {
            // 获取 hash 的所有字段
            let data: Vec<(String, String)> = spring_redis::redis::cmd("HGETALL")
                .arg(&key)
                .query_async(&mut redis)
                .await
                .context("Redis HGETALL failed")?;

            if !data.is_empty() {
                let mut ip = String::new();
                let mut request_count = 0i64;
                let mut first_seen = String::new();
                let mut last_seen = String::new();
                let mut block_until = String::new();

                for (field, value) in data {
                    match field.as_str() {
                        "ip" => ip = value,
                        "request_count" => request_count = value.parse().unwrap_or(0),
                        "first_seen" => first_seen = value,
                        "last_seen" => last_seen = value,
                        "block_until" => block_until = value,
                        _ => {}
                    }
                }

                blocked_ips.push(BlockedIp {
                    ip,
                    request_count,
                    first_seen,
                    last_seen,
                    block_until,
                });
            }
        }

        cursor = new_cursor;
        if cursor == 0 {
            break;
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
    let mut users = Vec::new();
    let mut cursor = 0u64;

    loop {
        let (new_cursor, keys): (u64, Vec<String>) = spring_redis::redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg("suspicious_user:*")
            .arg("COUNT")
            .arg(100)
            .query_async(&mut redis)
            .await
            .context("Redis SCAN suspicious_user failed")?;

        for key in keys {
            let data: Vec<(String, String)> = spring_redis::redis::cmd("HGETALL")
                .arg(&key)
                .query_async(&mut redis)
                .await
                .context("Redis HGETALL failed")?;

            if !data.is_empty() {
                let mut user_id = String::new();
                let mut user_name = String::new();
                let mut request_count = 0i64;
                let mut error_rate = 0.0f64;
                let mut window_start = String::new();
                let mut window_end = String::new();
                let mut risk_level = String::from("low");

                for (field, value) in data {
                    match field.as_str() {
                        "user_id" => user_id = value,
                        "user_name" => user_name = value,
                        "request_count" => request_count = value.parse().unwrap_or(0),
                        "error_rate" => error_rate = value.parse().unwrap_or(0.0),
                        "window_start" => window_start = value,
                        "window_end" => window_end = value,
                        "risk_level" => risk_level = value,
                        _ => {}
                    }
                }

                users.push(SuspiciousUser {
                    user_id,
                    user_name,
                    request_count,
                    error_rate,
                    window_start,
                    window_end,
                    risk_level,
                });
            }
        }

        cursor = new_cursor;
        if cursor == 0 {
            break;
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
    let mut stats = Vec::new();
    let mut cursor = 0u64;

    loop {
        let (new_cursor, keys): (u64, Vec<String>) = spring_redis::redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg("traffic:stats:*")
            .arg("COUNT")
            .arg(100)
            .query_async(&mut redis)
            .await
            .context("Redis SCAN traffic failed")?;

        for key in keys {
            let data: Vec<(String, String)> = spring_redis::redis::cmd("HGETALL")
                .arg(&key)
                .query_async(&mut redis)
                .await
                .context("Redis HGETALL failed")?;

            if !data.is_empty() {
                let mut metric_type = String::new();
                let mut metric_key = String::new();
                let mut value = 0i64;
                let mut window_start = String::new();
                let mut window_end = String::new();

                for (field, val) in data {
                    match field.as_str() {
                        "metric_type" => metric_type = val,
                        "metric_key" => metric_key = val,
                        "value" => value = val.parse().unwrap_or(0),
                        "window_start" => window_start = val,
                        "window_end" => window_end = val,
                        _ => {}
                    }
                }

                stats.push(TrafficStats {
                    metric_type,
                    metric_key,
                    value,
                    window_start,
                    window_end,
                });
            }
        }

        cursor = new_cursor;
        if cursor == 0 {
            break;
        }
    }

    Ok(Json(stats))
}

/// 获取限流配置
#[get("/api/stats/rate-limits")]
async fn get_rate_limits(Component(mut redis): Component<Redis>) -> Result<impl IntoResponse> {
    let mut configs = Vec::new();
    let mut cursor = 0u64;

    loop {
        let (new_cursor, keys): (u64, Vec<String>) = spring_redis::redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg("rate_limit:*")
            .arg("COUNT")
            .arg(100)
            .query_async(&mut redis)
            .await
            .context("Redis SCAN rate_limit failed")?;

        for key in keys {
            let data: Vec<(String, String)> = spring_redis::redis::cmd("HGETALL")
                .arg(&key)
                .query_async(&mut redis)
                .await
                .context("Redis HGETALL failed")?;

            if !data.is_empty() {
                let mut endpoint = String::new();
                let mut current_qps = 0i64;
                let mut avg_response_time = 0.0f64;
                let mut error_rate = 0.0f64;
                let mut suggested_limit = 0i64;
                let mut window_time = String::new();

                for (field, val) in data {
                    match field.as_str() {
                        "endpoint" => endpoint = val,
                        "current_qps" => current_qps = val.parse().unwrap_or(0),
                        "avg_response_time" => avg_response_time = val.parse().unwrap_or(0.0),
                        "error_rate" => error_rate = val.parse().unwrap_or(0.0),
                        "suggested_limit" => suggested_limit = val.parse().unwrap_or(0),
                        "window_time" => window_time = val,
                        _ => {}
                    }
                }

                configs.push(RateLimitConfig {
                    endpoint,
                    current_qps,
                    avg_response_time,
                    error_rate,
                    suggested_limit,
                    window_time,
                });
            }
        }

        cursor = new_cursor;
        if cursor == 0 {
            break;
        }
    }

    // 按 QPS 排序
    configs.sort_by(|a, b| b.current_qps.cmp(&a.current_qps));

    Ok(Json(configs))
}

/// 获取热门 URL
#[get("/api/stats/hot-urls")]
async fn get_hot_urls(Component(mut redis): Component<Redis>) -> Result<impl IntoResponse> {
    let mut urls = Vec::new();
    let mut cursor = 0u64;

    loop {
        let (new_cursor, keys): (u64, Vec<String>) = spring_redis::redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg("hot_url:*")
            .arg("COUNT")
            .arg(100)
            .query_async(&mut redis)
            .await
            .context("Redis SCAN hot_url failed")?;

        for key in keys {
            let data: Vec<(String, String)> = spring_redis::redis::cmd("HGETALL")
                .arg(&key)
                .query_async(&mut redis)
                .await
                .context("Redis HGETALL failed")?;

            if !data.is_empty() {
                let mut url_path = String::new();
                let mut request_count = 0i64;
                let mut avg_response_size = 0.0f64;
                let mut status_4xx_count = 0i64;
                let mut status_5xx_count = 0i64;
                let mut window_start = String::new();
                let mut window_end = String::new();

                for (field, val) in data {
                    match field.as_str() {
                        "url_path" => url_path = val,
                        "request_count" => request_count = val.parse().unwrap_or(0),
                        "avg_response_size" => avg_response_size = val.parse().unwrap_or(0.0),
                        "status_4xx_count" => status_4xx_count = val.parse().unwrap_or(0),
                        "status_5xx_count" => status_5xx_count = val.parse().unwrap_or(0),
                        "window_start" => window_start = val,
                        "window_end" => window_end = val,
                        _ => {}
                    }
                }

                urls.push(HotUrl {
                    url_path,
                    request_count,
                    avg_response_size,
                    status_4xx_count,
                    status_5xx_count,
                    window_start,
                    window_end,
                });
            }
        }

        cursor = new_cursor;
        if cursor == 0 {
            break;
        }
    }

    // 按请求数排序，取 Top 50
    urls.sort_by(|a, b| b.request_count.cmp(&a.request_count));
    urls.truncate(50);

    Ok(Json(urls))
}

