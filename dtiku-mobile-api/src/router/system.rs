use dtiku_base::service::system_config::SystemConfigService;
use serde::Serialize;
use spring_web::{
    axum::Json,
    error::Result,
    extractor::Component,
    get_api,
};
use schemars::JsonSchema;
#[derive(Debug, Serialize, JsonSchema)]
pub struct SystemConfigResponse {
    pub site_name: String,
    pub site_description: String,
    pub version: String,
}

/// GET /api/system/config
#[get_api("/api/system/config")]
async fn api_system_config(
    Component(sc): Component<SystemConfigService>,
) -> Result<Json<SystemConfigResponse>> {
    let _config = sc.load_config().await?;

    Ok(Json(SystemConfigResponse {
        site_name: "滴题库".to_string(),
        site_description: "公务员考试题库".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }))
}

