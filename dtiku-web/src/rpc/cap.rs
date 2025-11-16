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
pub struct VerifyReq {
    pub secret: String,
    #[serde(rename = "response")]
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyResult {
    pub success: bool,
}

#[post(url = get_captcha_url(), path = "/{site_key}/siteverify")]
async fn site_verify_req(
    site_key: String,
    #[body] req: VerifyReq,
) -> feignhttp::Result<VerifyResult> {
}

pub async fn site_verify(site_key: &str, site_secret: &str, token: &str) -> anyhow::Result<bool> {
    let result = site_verify_req(
        site_key.to_string(),
        VerifyReq {
            secret: site_secret.to_string(),
            token: token.to_string(),
        },
    )
    .await
    .context("site verify request failed")?;
    Ok(result.success)
}
