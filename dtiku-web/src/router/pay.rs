use super::Claims;
use crate::{
    query::pay::TradeCreateQuery,
    views::{pay::PayTradeCreateTemplate, GlobalVariables},
};
use anyhow::Context;
use askama::Template;
use dtiku_pay::{
    alipay_sdk_rust::biz::{self, BizContenter as _},
    service::pay_order::PayOrderService,
    Alipay,
};
use spring_web::{
    axum::{
        response::{Html, IntoResponse},
        Extension, Form,
    },
    error::Result,
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
    Form(trade): Form<TradeCreateQuery>,
) -> Result<impl IntoResponse> {
    println!("{trade:?}");
    ps.create_order(claims.user_id, trade.level, trade.pay_from)
        .await?;
    Ok("支付接口正在施工中...")
}

#[post("/pay/alipay/callback")]
async fn alipay_callback() -> Result<impl IntoResponse> {
    Ok("支付接口正在施工中...")
}

#[post("/pay/wechat/callback")]
async fn wechat_callback() -> Result<impl IntoResponse> {
    Ok("支付接口正在施工中...")
}
