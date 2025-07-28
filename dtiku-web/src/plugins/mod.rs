use serde::Deserialize;
use spring::config::Configurable;

pub mod grpc_client;

#[derive(Debug, Configurable, Deserialize)]
#[config_prefix = "grpc-client"]
pub struct GrpcClientConfig {
    pub(crate) embedding_url: String,
    pub(crate) artalk_url: String,
}

#[derive(Debug, Configurable, Deserialize)]
#[config_prefix = "web-dav-client"]
pub struct OpenListConfig {
    pub(crate) username: String,
    pub(crate) password: String,
}
