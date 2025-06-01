use super::Claims;
use crate::{
    query::pay::TradeCreateQuery,
    views::{
        pay::{PayRedirectTemplate, PayTradeCreateTemplate},
        GlobalVariables,
    },
};
use anyhow::Context;
use askama::Template;
use dtiku_pay::service::pay_order::PayOrderService;
use spring_web::{
    axum::{
        response::{Html, IntoResponse},
        Extension, Form,
    },
    error::{KnownWebError, Result},
    extractor::Component,
    get, post,
};

#[get("/pay/render")]
async fn render_pay(
    claims: Claims,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let t = PayTradeCreateTemplate {
        global,
        user_id: claims.user_id,
    };
    Ok(Html(t.render().context("render failed")?))
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
    let t = PayRedirectTemplate {
        global,
        qrcode_url,
        pay_from: trade.pay_from,
    };
    Ok(Html(t.render().context("render failed")?))
}

#[post("/pay/alipay/callback")]
async fn alipay_callback() -> Result<impl IntoResponse> {
    Ok("支付接口正在施工中...")
}

#[post("/pay/wechat/callback")]
async fn wechat_callback() -> Result<impl IntoResponse> {
    Ok("支付接口正在施工中...")
}
