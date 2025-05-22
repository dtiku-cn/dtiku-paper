use feignhttp::get;
use serde::Deserialize;
use std::collections::HashMap;

const ARTALK_URL: &str = "https://artalk.dtiku.cn/api/v2";

#[derive(Debug, Deserialize)]
pub struct StatsResult {
    pub data: HashMap<String, i32>,
}

/// https://docs.rs/feignhttp
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
