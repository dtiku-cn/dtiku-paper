use serde::Deserialize;
use spring::config::Configurable;

#[derive(Debug, Deserialize, Configurable)]
#[config_prefix = "huggingface"]
pub struct HfConfig {
    pub cache_dir: String,
}
