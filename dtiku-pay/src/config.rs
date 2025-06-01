use serde::Deserialize;
use spring::config::Configurable;

#[derive(Debug, Configurable, Deserialize)]
#[config_prefix = "pay"]
pub struct PayConfig {
    pub alipay_api_url: String,
    pub alipay_app_id: String,
    /// 支付宝根证书
    pub alipay_root_cert_sn: String,
    pub alipay_public_key: String,
    pub alipay_app_cert_sn: String,
    pub alipay_app_private_key: String,
    pub alipay_app_public_key: String,
}
