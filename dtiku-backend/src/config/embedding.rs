use std::net::{IpAddr, Ipv4Addr};

use serde::Deserialize;
use spring::config::Configurable;

#[derive(Debug, Deserialize, Configurable)]
#[config_prefix = "embedding"]
pub struct EmbeddingConfig {
    pub(crate) url: String,
}