use serde::Deserialize;
use spring::config::Configurable;

#[derive(Debug, Configurable, Deserialize)]
#[config_prefix = "web-dav-client"]
pub struct OpenListConfig {
    pub(crate) username: String,
    pub(crate) password: String,
    #[serde(default)]
    pub(crate) use_origin: bool,
}
