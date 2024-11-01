// You need to bring the type into scope to use it!!!
use strum::EnumMessage;

#[derive(PartialEq, Eq, Debug, EnumMessage)]
pub enum SystemConfigKey {
    #[strum(message = "联盟广告位脚本，show_ads开启时生效")]
    AdsScript,

    #[strum(message = "Raline评论配置")]
    RalineConfig,
}
