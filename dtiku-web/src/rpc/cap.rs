use anyhow::Context;
use feignhttp::post;
use serde::{Deserialize, Serialize};
use std::{env, sync::OnceLock};

static CAPTCHA_URL: OnceLock<String> = OnceLock::new();

/// 获取 Artalk API URL
fn get_captcha_url() -> &'static str {
    CAPTCHA_URL.get_or_init(|| {
        env::var("CAPTCHA_URL").unwrap_or_else(|_| "https://cap.dtiku.cn".to_string())
    })
}

#[derive(Debug, Serialize)]
struct VerifyReq<'a> {
    secret: &'a str,
    #[serde(rename = "response")]
    token: &'a str,
}

#[derive(Debug, Deserialize)]
struct VerifyResult {
    success: bool,
}

#[post(url = get_captcha_url(), path = "/{site_key}/siteverify")]
async fn site_verify_req<'a>(
    #[path("site_key")] site_key: &str,
    #[body] req: VerifyReq<'a>,
) -> feignhttp::Result<VerifyResult> {
}

pub async fn site_verify(site_key: &str, secret: &str, token: &str) -> anyhow::Result<bool> {
    let result = site_verify_req(site_key, VerifyReq { secret, token })
        .await
        .context("site verify request failed")?;
    Ok(result.success)
}
