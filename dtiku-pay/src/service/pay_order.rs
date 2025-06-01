use crate::{
    model::{pay_order, OrderLevel, PayFrom},
    Alipay, WechatPayClient,
};
use alipay_sdk_rust::{biz, response::TradePrecreateResponse};
use anyhow::Context;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DbConn};
use spring::{plugin::service::Service, tracing};
use std::net::IpAddr;
use wechat_pay_rust_sdk::{
    model::{H5Params, H5SceneInfo},
    response::H5Response,
};

#[derive(Clone, Service)]
pub struct PayOrderService {
    #[inject(component)]
    db: DbConn,
    #[inject(component)]
    alipay: Alipay,
    // #[inject(component)]
    // wechat: WechatPayClient,
}

impl PayOrderService {
    pub async fn create_order(
        &self,
        user_id: i32,
        level: OrderLevel,
        from: PayFrom,
        ip_addr: IpAddr,
    ) -> anyhow::Result<Option<String>> {
        let order = pay_order::ActiveModel {
            user_id: Set(user_id),
            level: Set(level),
            pay_from: Set(from),
            ..Default::default()
        }
        .insert(&self.db)
        .await
        .context("创建订单失败")?;

        let subject = format!("公考加油站{}会员", level.title());
        let out_trade_no = order.id.to_string();
        let amount = level.amount();
        let qrcode_url = match from {
            PayFrom::Alipay => {
                let mut biz_content = biz::TradePrecreateBiz::new();
                biz_content.set_subject(subject.into());
                biz_content.set_out_trade_no(out_trade_no.into());
                biz_content.set_total_amount(level.amount().into());
                let resp = self
                    .alipay
                    .trade_precreate(&biz_content)
                    .context("支付宝订单创建失败")?;
                let TradePrecreateResponse {
                    response,
                    alipay_cert_sn,
                    sign,
                } = resp;
                tracing::info!(
                    "alipay resp sign ==> {sign:?}, alipay_cert_sn ==> {alipay_cert_sn:?}"
                );
                response.qr_code
            }
            PayFrom::Wechat => {
                // let resp = self
                //     .wechat
                //     .h5_pay(H5Params::new(
                //         subject,
                //         out_trade_no,
                //         amount.into(),
                //         H5SceneInfo::new(
                //             ip_addr.to_string().as_str(),
                //             "公考加油站",
                //             "https://gwy.dtiku.cn",
                //         ),
                //     ))
                //     .await
                //     .context("微信订单创建失败")?;

                // let H5Response {
                //     h5_url,
                //     code,
                //     message,
                // } = resp;
                // tracing::info!("wechat pay resp code ==> {code:?}, message ==> {message:?}");
                // h5_url
                Some("https://pay.weixin.qq.com".to_string())
            }
        };
        Ok(qrcode_url)
    }
}
