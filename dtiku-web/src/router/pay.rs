use super::Claims;
use crate::{
    query::pay::TradeCreateQuery,
    views::{
        pay::{PayRedirectTemplate, PayTradeCreateTemplate, YsmPayNotify},
        GlobalVariables,
    },
};
use dtiku_pay::service::pay_order::PayOrderService;
use spring::tracing;
use spring_web::{
    axum::{response::IntoResponse, Extension, Form, Json},
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

#[post("/pay/wechat/callback")]
async fn wechat_pay_callback(body: String) -> Result<impl IntoResponse> {
    Ok(format!("支付接口正在施工中...\n回调数据：{body}"))
}

#[post("/pay/alipay/callback")]
async fn alipay_callback(body: String) -> Result<impl IntoResponse> {
    Ok(format!("支付接口正在施工中...\n回调数据：{body}"))
}

