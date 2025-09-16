use crate::{
    config::PayConfig,
    model::{pay_order, OrderLevel, PayFrom},
    Alipay, WechatPayClient,
};
use alipay_sdk_rust::{biz, response::TradePrecreateResponse};
use anyhow::{anyhow, Context};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DbConn};
use spring::{plugin::service::Service, tracing};
use wechat_pay_rust_sdk::{model::NativeParams, response::NativeResponse};

#[derive(Clone, Service)]
pub struct PayOrderService {
    #[inject(component)]
    db: DbConn,
    #[inject(component)]
    alipay: Option<Alipay>,
    #[inject(component)]
    wechat: Option<WechatPayClient>,
    #[inject(config)]
    config: PayConfig,
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
        let order_id = order.id;
        let amount = if self.config.test_pay_amount {
            1 // 1分钱
        } else {
            level.amount()
        };
        let qrcode_url = match from {
            PayFrom::Alipay => self.alipay(subject, order_id, amount).await?,
            PayFrom::Wechat => self.wechat_pay(subject, order_id, amount).await?,
        };
        Ok(qrcode_url)
    }

    async fn wechat_pay(
        &self,
        subject: String,
        order_id: i32,
        amount: i32,
    ) -> anyhow::Result<Option<String>> {
        let out_trade_no = format!("{:06}", order_id); // 微信“商户订单号”字符串规则校验最少6字节
        let wechat = self.wechat.clone();
        let resp = wechat
            .ok_or_else(|| anyhow!("暂不支持微信支付"))?
            .native_pay(NativeParams::new(subject, out_trade_no, amount.into()))
            .await
            .context("微信订单创建失败")?;
        let NativeResponse {
            code_url,
            code,
            message,
        } = resp;
        tracing::info!("wechat pay resp code ==> {code:?}, message ==> {message:?}");
        Ok(code_url)
    }

    async fn alipay(
        &self,
        subject: String,
        out_trade_no: i32,
        amount: i32,
    ) -> anyhow::Result<Option<String>> {
        let alipay = self.alipay.clone();
        let mut biz_content: biz::TradePrecreateBiz = biz::TradePrecreateBiz::new();
        biz_content.set_subject(subject.into());
        biz_content.set_out_trade_no(out_trade_no.into());
        biz_content.set_total_amount((amount / 100).into());
        let resp = alipay
            .ok_or_else(|| anyhow!("暂不支持支付宝"))?
            .trade_precreate(&biz_content)
            .context("支付宝订单创建失败")?;
        let resp_json = serde_json::to_value(&resp).context("支付宝响应出错")?;
        pay_order::ActiveModel {
            id: Set(out_trade_no),
            resp: Set(Some(resp_json)),
            ..Default::default()
        }
        .update(&self.db)
        .await
        .context("更新订单响应失败")?;
        let TradePrecreateResponse {
            response,
            alipay_cert_sn,
            sign,
        } = resp;
        tracing::info!("alipay resp sign ==> {sign:?}, alipay_cert_sn ==> {alipay_cert_sn:?}");
        Ok(response.qr_code)
    }
}
