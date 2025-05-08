use serde::Deserialize;
use spring::config::Configurable;

#[derive(Debug, Deserialize, Configurable)]
#[config_prefix = "huggingface"]
pub struct HfConfig {
    #[serde(default = "default_cache_dir")]
    pub cache_dir: String,
}

fn default_cache_dir() -> String {
    "../.hf-cache".to_string()
}
