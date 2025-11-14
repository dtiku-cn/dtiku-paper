use anyhow::Context;
use feignhttp::get;
use reqwest::header::HeaderMap;
use serde::Deserialize;
use std::{collections::HashMap, env, sync::OnceLock};

static ARTALK_URL: OnceLock<String> = OnceLock::new();

/// 获取 Artalk API URL
fn get_artalk_url() -> &'static str {
    ARTALK_URL.get_or_init(|| {
        env::var("ARTALK_URL").unwrap_or_else(|_| "https://artalk.dtiku.cn/api/v2".to_string())
    })
}

/// https://docs.rs/feignhttp
pub async fn auth_callback(
    headers: HeaderMap,
    provider: &str,
    raw_query: &str,
) -> anyhow::Result<String> {
    let base_url = get_artalk_url();
    let client = reqwest::Client::new();

    let mut req = client.get(format!("{base_url}/auth/{provider}/callback?{raw_query}"));
    // 转发原始 headers
    for (key, value) in headers.iter() {
        // 建议排除某些敏感头
        if key != "host" && key != "content-length" {
            req = req.header(key, value);
        }
    }

    let resp = req
        .send()
        .await
        .context("send auth_callback request failed")?;

    Ok(resp.text().await.context("parse response text failed")?)
}

#[derive(Debug, Deserialize)]
pub struct StatsResult {
    pub data: HashMap<String, i32>,
}

#[get("https://artalk.dtiku.cn/api/v2", path = "/stats/page_comment")]
async fn page_comment_req(page_keys: String) -> feignhttp::Result<StatsResult> {}

#[get("https://artalk.dtiku.cn/api/v2", path = "/stats/page_pv")]
async fn page_pv_req(page_keys: String) -> feignhttp::Result<StatsResult> {}

pub async fn page_comment(page_keys: &Vec<String>) -> HashMap<String, i32> {
    let result = page_comment_req(page_keys.join(",")).await;
    match result {
        Ok(res) => res.data,
        Err(_) => HashMap::new(),
    }
}

pub async fn page_pv(page_keys: &Vec<String>) -> HashMap<String, i32> {
    let result = page_pv_req(page_keys.join(",")).await;
    match result {
        Ok(res) => res.data,
        Err(_) => HashMap::new(),
    }
}
