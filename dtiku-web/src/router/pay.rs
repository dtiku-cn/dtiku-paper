use super::Claims;
use crate::{
    query::pay::TradeCreateQuery,
    views::{
        pay::{PayRedirectTemplate, PayTradeCreateTemplate},
        GlobalVariables,
    },
};
use anyhow::Context;
use dtiku_pay::{model::PayOrder, service::pay_order::PayOrderService};
use http::StatusCode;
use sea_orm::DbConn;
use serde_json::json;
use spring::tracing;
use spring_web::{
    axum::{http::header::HeaderMap, response::IntoResponse, Extension, Form, Json},
    error::{KnownWebError, Result},
    extractor::{Component, Path},
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
    let (order_id, qrcode_url) = ps
        .create_order(claims.user_id, trade.level, trade.pay_from)
        .await?;
    let qrcode_url =
        qrcode_url.ok_or_else(|| KnownWebError::internal_server_error("支付码生成失败"))?;
    Ok(PayRedirectTemplate {
        global,
        order_id,
        qrcode_url,
        pay_from: trade.pay_from,
    })
}

#[post("/pay/{order_id}/status")]
async fn pay_status(
    claims: Claims,
    Path(order_id): Path<i32>,
    Component(db): Component<DbConn>,
) -> Result<impl IntoResponse> {
    Ok(Json(
        PayOrder::find_order_status(&db, order_id, claims.user_id)
            .await
            .context("查询订单失败")?
            .ok_or_else(|| KnownWebError::not_found("订单不存在"))?,
    ))
}

/// https://pay.weixin.qq.com/doc/v3/merchant/4012791882
#[post("/pay/wechat/callback")]
async fn wechat_pay_callback(
    Component(p_service): Component<PayOrderService>,
    headers: HeaderMap,
    body: String,
) -> Result<impl IntoResponse> {
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

    let notify = match p_service
        .verify_signature(serial, timestamp, nonce, signature, &body)
        .await
        .context("verify_signature failed")
    {
        Err(e) => {
            tracing::error!(
                serial = serial,
                signature = signature,
                timestamp = timestamp,
                nonce = nonce,
                "微信支付回调验签失败: {e:#}"
            );
            return Ok((
                StatusCode::BAD_REQUEST,
                Json(json!({"code": "FAIL", "message": "验签失败"})),
            ));
        }
        Ok(notify) => notify,
    };

    if let Err(e) = p_service.notify_wechat_pay(&notify).await {
        tracing::error!("处理微信支付回调失败: {e:#}");
        return Ok((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"code": "FAIL", "message": "处理失败"})),
        ));
    }

    Ok((StatusCode::OK, Json("".into())))
}

#[post("/pay/alipay/callback")]
async fn alipay_callback(body: String) -> Result<impl IntoResponse> {
    tracing::warn!("支付接口正在施工中...\n回调数据：{body}");
    Ok("success")
}
