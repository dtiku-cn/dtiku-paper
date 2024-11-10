use chrono::NaiveDateTime;
use dtiku_base::model::{enums::SystemConfigKey, system_config::Model};
use serde::Serialize;
use strum::EnumMessage;

#[derive(Debug, Serialize)]
pub struct SystemConfig {
    pub id: Option<i32>,
    pub version: i32,
    pub key: SystemConfigKey,
    pub key_desc: &'static str,
    pub value: Option<String>,
    pub created: Option<NaiveDateTime>,
    pub modified: Option<NaiveDateTime>,
}

impl From<Model> for SystemConfig {
    fn from(value: Model) -> Self {
        Self {
            id: Some(value.id),
            version: value.version,
            key: value.key,
            key_desc: value.key.get_message().unwrap_or(""),
            value: Some(value.value),
            created: Some(value.created),
            modified: Some(value.modified),
        }
    }
}

impl From<SystemConfigKey> for SystemConfig {
    fn from(value: SystemConfigKey) -> Self {
        Self {
            id: None,
            version: 0,
            key: value,
            key_desc: value.get_message().unwrap_or(""),
            value: None,
            created: None,
            modified: None,
        }
    }
}
