use feignhttp::get;
use lazy_static::lazy_static;
use serde::Deserialize;
use spring::tracing::instrument;
use std::{collections::HashMap, env};

lazy_static! {
    static ref ARTALK_URL: String =
        env::var("ARTALK_URL").unwrap_or_else(|_| "https://artalk.dtiku.cn/api/v2".to_string());
}

/// https://docs.rs/feignhttp
#[instrument]
#[get(ARTALK_URL, path = "/auth/{provider}/callback?{raw_query}")]
pub async fn auth_callback(
    #[header] cookie: &str,
    #[path] provider: &str,
    #[param] raw_query: &str,
) -> feignhttp::Result<String> {
}

#[derive(Debug, Deserialize)]
pub struct StatsResult {
    pub data: HashMap<String, i32>,
}

#[get(ARTALK_URL, path = "/stats/page_comment")]
async fn page_comment_req(page_keys: String) -> feignhttp::Result<StatsResult> {}

#[get(ARTALK_URL, path = "/stats/page_pv")]
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
