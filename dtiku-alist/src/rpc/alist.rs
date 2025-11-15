use crate::plugins::OpenListConfig;
use anyhow::Context;
use feignhttp::post;
use serde::{Deserialize, Serialize};
use spring_redis::cache;
use std::env;
use std::sync::OnceLock;

static ALIST_URL: OnceLock<String> = OnceLock::new();
static STATIC_URL: OnceLock<String> = OnceLock::new();

/// 获取 Alist API URL
fn get_alist_url() -> &'static str {
    ALIST_URL.get_or_init(|| {
        env::var("ALIST_URL").unwrap_or_else(|_| "https://alist.dtiku.cn".to_string())
    })
}

/// 获取静态资源 URL
fn get_static_url() -> &'static str {
    STATIC_URL
        .get_or_init(|| env::var("STATIC_URL").unwrap_or_else(|_| "https://s.dtiku.cn".to_string()))
}

pub async fn get_file_path(raw_path: &str, config: &OpenListConfig) -> anyhow::Result<String> {
    let path = if raw_path.starts_with("/") {
        raw_path.to_string()
    } else {
        format!("/{raw_path}")
    };
    let token = get_token(&config.username, &config.password)
        .await?
        .ok_or_else(|| anyhow::anyhow!("获取token失败"))?;
    let resp = get_file_info(
        &token,
        FileReq {
            path: path.clone(),
            ..Default::default()
        },
    )
    .await?;

    let static_url = get_static_url();
    Ok(format!("{static_url}/d{path}?sign={}", resp.data.sign))
}

#[cache("alist-token", expire = 170000)] // 默认48小时过期，所以过期时间设置为略小于172800
async fn get_token(username: &str, password: &str) -> anyhow::Result<Option<String>> {
    let r = login(LoginReq { username, password })
        .await
        .context("login get token failed");
    r.map(|r| Some(r.data.token))
}

#[post(url = get_alist_url(), path = "/api/auth/login")]
async fn login<'a>(#[body] req: LoginReq<'a>) -> feignhttp::Result<ArtalkResult<LoginResp>> {}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoginReq<'a> {
    pub username: &'a str,
    pub password: &'a str,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoginResp {
    pub token: String,
}

#[post(url = get_alist_url(), path = "/api/fs/get")]
async fn get_file_info(
    #[header] authorization: &str,
    #[body] req: FileReq,
) -> feignhttp::Result<ArtalkResult<FileResp>> {
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileReq {
    pub path: String,
    pub password: String,
    pub page: Option<i64>,
    #[serde(rename = "per_page")]
    pub per_page: Option<i64>,
    pub refresh: Option<bool>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtalkResult<T> {
    pub code: i64,
    pub message: String,
    pub data: T,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileResp {
    pub name: String,
    pub size: i64,
    pub is_dir: bool,
    pub modified: String,
    pub created: String,
    pub sign: String,
    pub raw_url: String,
    pub provider: String,
}
