use std::{
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    config::PayConfig,
    model::{pay_order, OrderLevel, PayFrom},
    Alipay, WechatPayClient,
};
use alipay_sdk_rust::{biz, response::TradePrecreateResponse};
use anyhow::{anyhow, Context};
use maplit::btreemap;
use rand::{distr::Alphanumeric, Rng as _};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DbConn};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use spring::{config::ConfigRef, plugin::service::Service, tracing};
use wechat_pay_rust_sdk::{model::NativeParams, response::NativeResponse};

#[derive(Clone, Service)]
pub struct PayOrderService {
    #[inject(component)]
    db: DbConn,
    #[inject(component)]
    alipay: Option<Alipay>,
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
            PayFrom::Alipay => self.yishoumi_alipay(subject, order_id, amount).await?, //self.alipay(subject, order_id, amount)?,
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

    async fn yishoumi_alipay(
        &self,
        subject: String,
        order_id: i32,
        amount: i32,
    ) -> anyhow::Result<Option<String>> {
        let PayConfig {
            yishoumi_appid,
            yishoumi_notify_url,
            yishoumi_nopay_url,
            yishoumi_callback_url,
            yishoumi_secret,
            ..
        } = &*self.pay_config;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let rand_string: String = rand::rng()
            .sample_iter(&Alphanumeric)
            .take(6)
            .map(char::from)
            .collect();
        let out_trade_no = format!("{:06}", order_id); // 字符串规则校验最少6字节

        let sign = Self::ysm_sign_hash(
            btreemap! {
                "appid"=>yishoumi_appid.to_string(),
                "mch_orderid"=>out_trade_no.to_string(),
                "description"=>subject.to_string(),
                "total"=> amount.to_string(),
                "payType"=> 11.to_string(),
                "notify_url"=> yishoumi_notify_url.to_string(),
                "nopay_url"=> yishoumi_nopay_url.to_string(),
                "callback_url"=> yishoumi_callback_url.to_string(),
                "time"=> timestamp.to_string(),
                "nonce_str"=>rand_string.to_string(),
            },
            yishoumi_secret,
        );

        let resp = reqwest::Client::new()
            .post("https://www.yishoumi.cn/u/payment")
            .json(&json!({
                "appid":yishoumi_appid,
                "mch_orderid":out_trade_no,
                "description":subject,
                "total": amount,
                "payType": 11,
                "notify_url": yishoumi_notify_url,
                "nopay_url": yishoumi_nopay_url,
                "callback_url": yishoumi_callback_url,
                "time": timestamp,
                "nonce_str":rand_string,
                "sign":sign,
            }))
            .send()
            .await
            .context("发起支付失败")?;

        #[derive(Debug, Deserialize)]
        struct Resp {
            code: i32,
            msg: String,
            #[serde(default, rename = "ordeid")]
            order_id: String,
            #[serde(default)]
            sign: String,
            #[serde(default)]
            url: String,
        }

        tracing::info!("支付返回状态: {}", resp.status());

        let r = resp
            .json::<Value>()
            .await
            .context("解析支付服务响应体失败")?;

        pay_order::ActiveModel {
            id: Set(order_id),
            resp: Set(Some(r.clone())),
            ..Default::default()
        }
        .update(&self.db)
        .await
        .context("保存支付订单失败")?;

        let r = serde_json::from_value::<Resp>(r).context("解析支付服务响应体JSON失败")?;

        tracing::info!(order_id = r.order_id, sign = r.sign, "支付返回: {}", r.msg);
        if r.code == 0 {
            Ok(Some(r.url))
        } else {
            Err(anyhow!("支付失败:{}", r.msg))
        }
    }

    #[inline]
    fn ysm_sign_hash(data: BTreeMap<&'static str, String>, secret: &str) -> String {
        // BTreeMap 本身就是有序的，等价于 PHP 的 ksort
        let mut parts = Vec::new();

        for (key, value) in &data {
            if *key == "hash" || value.is_empty() {
                continue;
            }
            parts.push(format!("{}={}", key, value));
        }

        let str_to_sign = format!("{}{}", parts.join("&"), secret);

        // sha256
        let mut hasher = Sha256::new();
        hasher.update(str_to_sign.as_bytes());
        let result = hasher.finalize();

        // 转 hex 字符串
        hex::encode(result)
    }

    fn alipay(
        &self,
        subject: String,
        out_trade_no: i32,
        amount: i32,
    ) -> anyhow::Result<Option<String>> {
        let alipay = self.alipay.clone();
        let mut biz_content: biz::TradePrecreateBiz = biz::TradePrecreateBiz::new();
        biz_content.set_subject(subject.into());
        biz_content.set_out_trade_no(out_trade_no.into());
        biz_content.set_total_amount(amount.into());
        let resp = alipay
            .ok_or_else(|| anyhow!("暂不支持支付宝"))?
            .trade_precreate(&biz_content)
            .context("支付宝订单创建失败")?;
        let TradePrecreateResponse {
            response,
            alipay_cert_sn,
            sign,
        } = resp;
        tracing::info!("alipay resp sign ==> {sign:?}, alipay_cert_sn ==> {alipay_cert_sn:?}");
        Ok(response.qr_code)
    }
}
