use std::net::{IpAddr, Ipv4Addr};

use serde::Deserialize;
use spring::config::Configurable;

#[derive(Debug, Deserialize, Configurable)]
#[config_prefix = "huggingface"]
pub struct HfConfig {
    #[serde(default = "default_cache_dir")]
    pub cache_dir: String,

    #[serde(default = "default_binding")]
    pub(crate) binding: IpAddr,

    #[serde(default = "default_port")]
    pub(crate) port: u16,
}

fn default_cache_dir() -> String {
    "../.hf-cache".to_string()
}

fn default_binding() -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))
}

fn default_port() -> u16 {
    8080
}