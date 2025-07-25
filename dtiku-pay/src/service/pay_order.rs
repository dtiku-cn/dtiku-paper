use crate::{
    model::{pay_order, OrderLevel, PayFrom},
    Alipay, WechatPayClient,
};
use alipay_sdk_rust::{biz, response::TradePrecreateResponse};
use anyhow::Context;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DbConn};
use spring::{plugin::service::Service, tracing};
use wechat_pay_rust_sdk::{model::NativeParams, response::NativeResponse};

#[derive(Clone, Service)]
pub struct PayOrderService {
    #[inject(component)]
    db: DbConn,
    #[inject(component)]
    alipay: Alipay,
    #[inject(component)]
    wechat: WechatPayClient,
}

impl PayOrderService {
    pub async fn create_order(
        &self,
        user_id: i32,
        level: OrderLevel,
        from: PayFrom,
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
        let out_trade_no = format!("{:06}", order.id); // 微信“商户订单号”字符串规则校验最少6字节
        let amount = level.amount();
        let qrcode_url = match from {
            PayFrom::Alipay => {
                let mut biz_content = biz::TradePrecreateBiz::new();
                biz_content.set_subject(subject.into());
                biz_content.set_out_trade_no(out_trade_no.into());
                biz_content.set_total_amount(amount.into());
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
                let resp = self
                    .wechat
                    .native_pay(NativeParams::new(subject, out_trade_no, amount.into()))
                    .await
                    .context("微信订单创建失败")?;

                let NativeResponse {
                    code_url,
                    code,
                    message,
                } = resp;
                tracing::info!("wechat pay resp code ==> {code:?}, message ==> {message:?}");
                code_url
            }
        };
        Ok(qrcode_url)
    }
}
