use std::collections::HashMap;

use crate::{
    config::PayConfig,
    model::{pay_order, OrderLevel, PayFrom, PayOrder},
    Alipay, WechatPayClient,
};
use alipay_sdk_rust::{
    biz::{self, BizContenter},
    response::TradePrecreateResponse,
};
use anyhow::{anyhow, Context};
use sea_orm::{
    prelude::DateTime, sqlx::types::chrono::Local, ActiveModelTrait, ActiveValue::Set, DbConn,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use spring::{plugin::service::Service, tracing};
use wechat_pay_rust_sdk::{
    model::NativeParams,
    response::{NativeResponse, ResponseTrait},
};

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
        let mut biz_content = biz::TradePrecreateBiz::new();
        biz_content.set_subject(subject.into());
        biz_content.set_out_trade_no(out_trade_no.into());
        biz_content.set_total_amount((amount as f64 / 100.0).into());
        biz_content.set("notify_url", self.config.alipay_callback_url.clone().into());
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

    pub async fn query_alipay_order(&self, model: pay_order::Model) -> anyhow::Result<()> {
        let order_id = model.id;
        let alipay = self.alipay.clone();
        let mut biz_content = biz::TradeQueryBiz::new();
        biz_content.set_out_trade_no(order_id.into());

        let resp = alipay
            .ok_or_else(|| anyhow!("暂不支持支付宝"))?
            .trade_query(&biz_content)
            .context("支付宝订单查询失败")?;

        if resp.response.trade_status == Some("TRADE_SUCCESS".to_owned()) {
            let now = Local::now().naive_local();

            pay_order::ActiveModel {
                id: Set(order_id),
                confirm: Set(Some(now)),
                resp: Set(Some(
                    serde_json::to_value(resp).context("resp to json failed")?,
                )),
                ..Default::default()
            }
            .update(&self.db)
            .await
            .with_context(|| format!("update_pay_order({order_id}) failed"))?;

            Ok(())
        } else {
            Err(anyhow!("订单状态不成功: {:?}", resp.response.trade_status))
        }
    }

    pub async fn query_wechat_order(&self, model: pay_order::Model) -> anyhow::Result<()> {
        match self.wechat.clone() {
            Some(wechat) => {
                let order_id = model.id;
                let mchid = &wechat.mch_id;
                let resp = wechat
                    .get_pay::<WechatPayOrderResp>(&format!(
                        "/v3/pay/transactions/out-trade-no/{order_id:06}?mchid={mchid}"
                    ))
                    .await
                    .context("微信订单查询失败")?;

                tracing::info!(
                    "order#{order_id} confirm state: {}({})",
                    resp.trade_state,
                    resp.trade_state_desc
                );

                if resp.trade_state == "SUCCESS" {
                    let now = Local::now().naive_local();

                    pay_order::ActiveModel {
                        id: Set(order_id),
                        confirm: Set(Some(now)),
                        resp: Set(Some(
                            serde_json::to_value(resp).context("resp to json failed")?,
                        )),
                        ..Default::default()
                    }
                    .update(&self.db)
                    .await
                    .with_context(|| format!("update_pay_order({order_id}) failed"))?;

                    Ok(())
                } else {
                    Err(anyhow!(
                        "订单状态不成功: {}({})",
                        resp.trade_state,
                        resp.trade_state_desc
                    ))
                }
            }
            None => Err(anyhow!("暂不支持微信支付")),
        }
    }

    pub async fn find_wait_confirm_after(
        &self,
        after_time: DateTime,
    ) -> anyhow::Result<Vec<pay_order::Model>> {
        PayOrder::find_wait_confirm_after(&self.db, after_time).await
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WechatPayOrderResp {
    pub appid: String,
    pub mchid: String,
    pub trade_state: String,
    pub trade_state_desc: String,
    pub out_trade_no: String,
    pub transaction_id: String,
    pub trade_type: String,
    pub success_time: String,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl ResponseTrait for WechatPayOrderResp {}
