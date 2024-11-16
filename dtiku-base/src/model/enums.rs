use sea_orm::prelude::StringLen;
use sea_orm::DeriveActiveEnum;
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumIter, EnumMessage, EnumString};

#[derive(
    Copy,
    Clone,
    Hash,
    PartialEq,
    Eq,
    Debug,
    Serialize,
    Deserialize,
    DeriveActiveEnum,
    EnumMessage,
    EnumIter,
    AsRefStr,
    EnumString,
)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::None)",
    rename_all = "camelCase"
)]
pub enum SystemConfigKey {
    #[strum(message = "联盟广告位脚本，show_ads开启时生效")]
    AdsScript,

    #[strum(message = "Raline评论配置")]
    RalineConfig,
}

#[derive(
    Copy,
    Clone,
    Hash,
    PartialEq,
    Eq,
    Debug,
    Serialize,
    Deserialize,
    DeriveActiveEnum,
    EnumMessage,
    EnumIter,
    AsRefStr,
    EnumString,
)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::None)",
    rename_all = "camelCase"
)]
pub enum ScheduleTaskType {
    #[strum(message = "同步粉笔数据")]
    FenbiSync,
}
