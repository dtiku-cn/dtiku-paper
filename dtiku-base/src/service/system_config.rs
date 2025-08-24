use crate::model::{enums::SystemConfigKey, SystemConfig};
use itertools::Itertools as _;
use sea_orm::DbConn;
use serde::Deserialize;
use spring::config::Configurable;
use spring::plugin::service::Service;

#[derive(Debug, Clone, Service)]
pub struct SystemConfigService {
    #[inject(component)]
    db: DbConn,
    #[inject(config)]
    config: DefaultConfig,
}

macro_rules! gen_config_getters {
    (
        $(
            ($fn_name:ident, $key:ident, $ret:ty)
        ),* $(,)?
    ) => {
        #[derive(Debug, Clone, Configurable, Deserialize)]
        #[config_prefix = "site"]
        pub struct DefaultConfig {
            $(
                pub $fn_name: $ret,
            )*
        }

        #[derive(Debug, Clone)]
        pub struct Config {
            $(
                pub $fn_name: $ret,
            )*
        }

        impl SystemConfigService {
            pub async fn load_config(&self) -> anyhow::Result<Config> {
                Ok(Config{
                    $(
                        $fn_name: self.$fn_name().await?,
                    )*
                })
            }
            $(
                pub async fn $fn_name(&self) -> anyhow::Result<$ret> {
                    match SystemConfig::decode_cached_value(&self.db, SystemConfigKey::$key).await? {
                        Some(v) => Ok(v),
                        None => Ok(self.config.$fn_name.clone())
                    }
                }
            )*
        }
    };
}

gen_config_getters! {
    (navbar_brand, NavbarBrand, String),
    (site_title, SiteTitle, String),
    (show_ads, ShowAds, bool),
    (ads_script, AdsScript, String),
    (show_comments, ShowComments, bool),
    (show_visitors, ShowVisitors, bool),
    (show_solution, ShowSolution, bool),
    (show_vendor, ShowVendor, bool),
    (use_cdn_asset, UseCdnAsset, bool),
    (cdn_assets, CdnAssets, serde_json::Value),
    (alert_message_key, AlertMessageKey, String),
    (global_alert_message, GlobalAlertMessage, String),
    (analytics_script, AnalyticsScript, String),
    (global_style, GlobalStyle, String),
    (global_head_files, GlobalHeadFiles, String),
    (iconfont_js_url, IconfontJsUrl, String),
    (block_ip_pv_threshold, BlockIpPvThreshold, u32),
    (block_user_agents, BlockUserAgents, String),
    (seo_user_agents, SeoUserAgents, String),
    (ip_blacklist, IpBlacklist, String),
}

impl SystemConfigService {
    pub async fn split_seo_user_agents(&self) -> Vec<String> {
        self.seo_user_agents()
            .await
            .ok()
            .unwrap_or_else(|| "Googlebot,Bingbot,Baiduspider,Sogou".to_string())
            .split(',')
            .map(|str| str.to_string())
            .collect_vec()
    }
}
