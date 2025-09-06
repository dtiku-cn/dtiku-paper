use crate::{
    config::PayConfig,
    model::{pay_order, OrderLevel, PayFrom},
    service::alipay,
    WechatPayClient,
};
use anyhow::{anyhow, Context};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DbConn};
use serde::Deserialize;
use spring::{config::ConfigRef, plugin::service::Service, tracing};
use wechat_pay_rust_sdk::{model::NativeParams, response::NativeResponse};

#[derive(Clone, Service)]
pub struct PayOrderService {
    #[inject(component)]
    db: DbConn,
    #[inject(component)]
    wechat: Option<WechatPayClient>,
    pay_config: ConfigRef<PayConfig>,
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
        let amount = level.amount();
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
        let PayConfig {
            alipay_api_url,
            alipay_app_id,
            alipay_public_key,
            alipay_app_private_key,
            alipay_app_public_key,
            ..
        } = &*self.pay_config;
        let alipay = alipay::qrcode::AlipayService::new(
            alipay_api_url,
            alipay_app_id,
            "/pay/notify",
            alipay_app_private_key,
        )
        .set_subject(&subject)
        .set_out_trade_no(&out_trade_no.to_string())
        .set_total_fee(amount.into());
        let resp = alipay.do_pay();

        pay_order::ActiveModel {
            id: Set(out_trade_no),
            resp: Set(Some(resp.clone())),
            ..Default::default()
        }
        .update(&self.db)
        .await
        .context("更新失败")?;

        #[derive(Default, Debug, Clone, PartialEq, Deserialize)]
        pub struct AlipayResp {
            pub alipay_trade_precreate_response: AlipayTradePrecreateResponse,
            pub sign: String,
        }

        #[derive(Default, Debug, Clone, PartialEq, Deserialize)]
        pub struct AlipayTradePrecreateResponse {
            pub code: String,
            pub msg: String,

            #[serde(default)]
            pub out_trade_no: Option<String>,

            #[serde(default)]
            pub qr_code: Option<String>,

            #[serde(default)]
            pub sub_code: Option<String>,

            #[serde(default)]
            pub sub_msg: Option<String>,
        }

        let resp = serde_json::from_value::<AlipayResp>(resp).context("解析json成功")?;

        Ok(resp.alipay_trade_precreate_response.qr_code)
    }
}
