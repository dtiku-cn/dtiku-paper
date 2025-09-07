//! * https://opendocs.alipay.com/open/270/01didh?pathHash=a6ccbe9a&ref=api
//! * https://opendocs.alipay.com/common/02mriz
//! * https://opendocs.alipay.com/support/01rauw
//! * https://github.com/wandercn/alipay_sdk_rust
//!
//! * https://pay.weixin.qq.com/doc/v3/merchant/4012791832
mod config;
pub mod model;
pub mod service;

pub use alipay_sdk_rust;
use alipay_sdk_rust::pay::{PayClient, Payer};
use config::PayConfig;
use derive_more::derive::Deref;
use spring::{
    app::AppBuilder,
    async_trait,
    config::ConfigRegistry,
    plugin::{MutableComponentRegistry, Plugin},
};
use std::sync::Arc;
use wechat_pay_rust_sdk::pay::WechatPay;

pub struct PayPlugin;

#[async_trait]
impl Plugin for PayPlugin {
    async fn build(&self, app: &mut AppBuilder) {
        let conf = app.get_config::<PayConfig>().expect("支付配置获取失败");

        if conf.alipay_enable {
            let alipay = PayClient::builder()
                .api_url(&conf.alipay_api_url)
                .app_id(&conf.alipay_app_id)
                .alipay_root_cert_sn(&conf.alipay_root_cert_sn)
                .alipay_public_key(&conf.alipay_public_key)
                .app_cert_sn(&conf.alipay_app_cert_sn)
                .charset_utf8()
                .format_json()
                .private_key(&conf.alipay_app_private_key)
                .public_key(&conf.alipay_app_public_key)
                .sign_type_rsa2()
                .version_1_0()
                .build()
                .expect("build alipay client failed");

            app.add_component(Alipay(Arc::new(alipay)));
        }

        if conf.wechat_pay_enable {
            let wechat_pay = WechatPay::from_env();
            app.add_component(WechatPayClient(Arc::new(wechat_pay)));
        }
    }
}

#[derive(Clone, Deref)]
pub struct Alipay(Arc<dyn Payer + Send + Sync>);

#[derive(Clone, Deref)]
pub struct WechatPayClient(Arc<WechatPay>);

#[cfg(test)]
mod tests {
    use alipay_sdk_rust::cert;

    const APP_CERT_SN_FILE: &str = "/Users/holmofy/Documents/支付宝开放平台密钥工具/证书20250907201037/appCertPublicKey_2021005188688168.crt";
    const ALIPAY_ROOT_CERT_FILE: &str =
        "/Users/holmofy/Documents/支付宝开放平台密钥工具/证书20250907201037/alipayRootCert.crt";
    const ALIPAY_CERT_PUBLIC_KEY_RSA2_FILE: &str = "/Users/holmofy/Documents/支付宝开放平台密钥工具/证书20250907201037/alipayCertPublicKey_RSA2.crt";

    // const APP_CERT_SN_FILE: &str = "/Users/holmofy/Documents/支付宝开放平台密钥工具/证书20250531174800/沙箱应用/appPublicCert.crt";
    // const ALIPAY_ROOT_CERT_FILE: &str =
    //     "/Users/holmofy/Documents/支付宝开放平台密钥工具/证书20250531174800/沙箱应用/alipayRootCert.crt";
    // const ALIPAY_CERT_PUBLIC_KEY_RSA2_FILE: &str = "/Users/holmofy/Documents/支付宝开放平台密钥工具/证书20250531174800/沙箱应用/alipayPublicCert.crt";

    #[test]
    fn test_sn() {
        match cert::get_cert_sn(APP_CERT_SN_FILE) {
            Ok(sn) => {
                println!("app_cert_sn: {}", sn)
            }
            Err(err) => {
                println!("get app_cert_sn faild: {}", err)
            }
        }

        match cert::get_root_cert_sn(ALIPAY_ROOT_CERT_FILE) {
            Ok(sn) => {
                println!("alipay_root_cert_sn : {}", sn)
            }
            Err(err) => {
                println!("get alipay_root_cert_sn faild: {}", err)
            }
        }
        match cert::get_public_key_with_path(ALIPAY_CERT_PUBLIC_KEY_RSA2_FILE) {
            Ok(sn) => {
                println!("alipay_cert_public_key : {}", sn)
            }
            Err(err) => {
                println!("faild: {}", err)
            }
        }
        match cert::get_public_key_with_path(APP_CERT_SN_FILE) {
            Ok(sn) => {
                println!("app_cert_public_key : {}", sn)
            }
            Err(err) => {
                println!("faild: {}", err)
            }
        }
    }
}
