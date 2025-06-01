use super::Claims;
use crate::query::pay::TradeCreateQuery;
use anyhow::Context;
use dtiku_pay::{
    alipay_sdk_rust::biz::{self, BizContenter as _},
    Alipay,
};
use spring_web::{
    axum::{
        response::{Html, IntoResponse},
        Form,
    },
    error::Result,
    extractor::Component,
    get, post,
};

#[get("/pay/render")]
async fn render_pay() -> Result<impl IntoResponse> {
    Ok(Html(""))
}

#[get("/pay/trade")]
async fn create_trade(
    claims: Claims,
    Component(alipay): Component<Alipay>,
    Form(trade): Form<TradeCreateQuery>,
) -> Result<impl IntoResponse> {
    let out_trade_no = chrono::Utc::now().timestamp_nanos().to_string();
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string();
    let mut biz_content = biz::TradeCreateBiz::new();
    biz_content.set_subject("huawei Mate50".into());
    biz_content.set_out_trade_no(out_trade_no.into()); // "1620630871769533112"
    biz_content.set_total_amount("5".into());
    biz_content.set_buyer_id("2088722069264875".into());
    biz_content.set("Timestamp", timestamp.into());
    let res = alipay.trade_create(&biz_content).context("订单创建失败")?;
    println!("{}", serde_json::to_string(&res).context("to json failed")?);
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
