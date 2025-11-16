use serde::Deserialize;
use spring::config::Configurable;

pub mod grpc_client;

#[derive(Debug, Configurable, Deserialize)]
#[config_prefix = "grpc-client"]
pub struct GrpcClientConfig {
    pub(crate) embedding_url: String,
    pub(crate) artalk_url: String,
}

#[derive(Debug, Clone, Configurable, Deserialize)]
#[config_prefix = "dtiku"]
pub struct DtikuConfig {
    #[serde(default)]
    pub(crate) strip_prefix: String,
    #[serde(default)]
    pub(crate) cap_site_key: String,
    #[serde(default)]
    pub(crate) cap_secret_key: String,
    #[serde(default)]
    pub(crate) cap_custom_wasm_url: Option<String>,
}
