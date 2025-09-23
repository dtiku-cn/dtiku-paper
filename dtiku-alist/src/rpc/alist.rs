use crate::plugins::OpenListConfig;
use anyhow::Context;
use spring_redis::cache;
use feignhttp::post;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::env;

lazy_static! {
    static ref ALIST_URL: String =
        env::var("ALIST_URL").unwrap_or_else(|_| "https://alist.dtiku.cn".to_string());
    static ref STATIC_URL: String =
        env::var("STATIC_URL").unwrap_or_else(|_| "https://s.dtiku.cn".to_string());
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

    let static_url = STATIC_URL.as_str();
    Ok(format!("{static_url}/d{path}?sign={}", resp.data.sign))
}

#[cache("alist-token", expire = 170000)] // 默认48小时过期，所以过期时间设置为略小于172800
async fn get_token(username: &str, password: &str) -> anyhow::Result<Option<String>> {
    let r = login(LoginReq { username, password })
        .await
        .context("login get token failed");
    r.map(|r| Some(r.data.token))
}

#[post(ALIST_URL, path = "/api/auth/login")]
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

#[post(ALIST_URL, path = "/api/fs/get")]
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
