//! * https://opendocs.alipay.com/open/270/01didh?pathHash=a6ccbe9a&ref=api
//! * https://opendocs.alipay.com/common/02mriz
//! * https://opendocs.alipay.com/support/01rauw
//! * https://github.com/wandercn/alipay_sdk_rust
//!
//! * https://pay.weixin.qq.com/doc/v3/merchant/4012791832
mod config;
pub mod model;
pub mod service;

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

        if conf.wechat_pay_enable {
            let wechat_pay = WechatPay::from_env();
            app.add_component(WechatPayClient(Arc::new(wechat_pay)));
        }
    }
}

#[derive(Clone, Deref)]
pub struct WechatPayClient(Arc<WechatPay>);
