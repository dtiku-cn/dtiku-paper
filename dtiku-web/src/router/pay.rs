use super::Claims;
use crate::{
    query::pay::TradeCreateQuery,
    views::{
        pay::{PayRedirectTemplate, PayTradeCreateTemplate},
        GlobalVariables,
    },
};
use dtiku_pay::service::pay_order::PayOrderService;
use spring::tracing;
use spring_web::{
    axum::{http::header::HeaderMap, response::IntoResponse, Extension, Form},
    error::{KnownWebError, Result},
    extractor::Component,
    get, post,
};

#[get("/pay/render")]
async fn render_pay(
    claims: Claims,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    Ok(PayTradeCreateTemplate {
        global,
        user_id: claims.user_id,
    })
}

#[post("/pay/create")]
async fn create_trade(
    claims: Claims,
    Component(ps): Component<PayOrderService>,
    Extension(global): Extension<GlobalVariables>,
    Form(trade): Form<TradeCreateQuery>,
) -> Result<impl IntoResponse> {
    let qrcode_url = ps
        .create_order(claims.user_id, trade.level, trade.pay_from)
        .await?
        .ok_or_else(|| KnownWebError::internal_server_error("支付码生成失败"))?;
    Ok(PayRedirectTemplate {
        global,
        qrcode_url,
        pay_from: trade.pay_from,
    })
}

/// https://pay.weixin.qq.com/doc/v3/merchant/4012791882
#[post("/pay/wechat/callback")]
async fn wechat_pay_callback(headers: HeaderMap, body: String) -> Result<impl IntoResponse> {
    let serial = headers
        .get("Wechatpay-Serial")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    let signature = headers
        .get("Wechatpay-Signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    let timestamp = headers
        .get("Wechatpay-Timestamp")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    let nonce = headers
        .get("Wechatpay-Nonce")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();

    tracing::warn!(
        serial = serial,
        signature = signature,
        timestamp = timestamp,
        nonce = nonce,
        "支付接口正在施工中...\n回调数据：{headers:?}{body}"
    );
    Ok("<xml><return_code><![CDATA[SUCCESS]]></return_code><return_msg><![CDATA[OK]]></return_msg></xml>")
}

#[post("/pay/alipay/callback")]
async fn alipay_callback(body: String) -> Result<impl IntoResponse> {
    tracing::warn!("支付接口正在施工中...\n回调数据：{body}");
    Ok("success")
}
