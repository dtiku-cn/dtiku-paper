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
    #[strum(message = "标题栏Brand名")]
    NavbarBrand,

    #[strum(message = "网站标题")]
    SiteTitle,

    #[strum(message = "显示广告位")]
    ShowAds,

    #[strum(message = "联盟广告位脚本，show_ads开启时生效")]
    AdsScript,

    #[strum(message = "显示评论")]
    ShowComments,

    #[strum(message = "Artalk评论配置")]
    CommentConfig,

    #[strum(message = "显示页面访问量")]
    ShowVisitors,

    #[strum(message = "显示题目答案")]
    ShowSolution,

    #[strum(message = "显示数据来源")]
    ShowVendor,

    #[strum(message = "使用CDN资源")]
    UseCdnAsset,

    #[strum(message = "CDN资源配置")]
    CdnAssets,

    #[strum(
        message = "全局告警消息的Key，用户cookie中含有这个值说明用户关闭了广告，下次就不返回广告了"
    )]
    AlertMessageKey,

    #[strum(message = "全局告警消息")]
    GlobalAlertMessage,

    #[strum(message = "埋点统计脚本,详见百度统计,谷歌统计等")]
    AnalyticsScript,

    #[strum(message = "全局CSS样式")]
    GlobalStyle,

    #[strum(message = "全局外部头文件")]
    GlobalHeadFiles,

    #[strum(message = "iconfont.js CDN配置")]
    IconfontJsUrl,

    #[strum(message = "一些耗资源的内容，需要反蜘蛛爬虫，agent用逗号隔开")]
    BlockUserAgents,

    #[strum(message = "对SEO有帮助的爬虫，开放登录，agent用逗号隔开")]
    SeoUserAgents,

    #[strum(message = "ip黑名单，用逗号隔开")]
    IpBlacklist,
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
