use crate::router::Claims;
use anyhow::Context;
use dtiku_pay::{model::{PayOrder, OrderLevel, PayFrom}, service::pay_order::PayOrderService};
use serde::{Deserialize, Serialize};
use spring_web::{
    axum::{response::IntoResponse, Json},
    error::{KnownWebError, Result},
    extractor::{Component, Path},
    get, post,
};
use sea_orm::DbConn;

#[derive(Debug, Deserialize)]
pub struct PayCreateRequest {
    pub product_name: String,
    pub product_description: Option<String>,
    pub amount: i64,
    pub payment_method: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PayOrderResponse {
    pub order_id: String,
    pub product_name: String,
    pub amount: i64,
    pub status: String,
    pub qrcode_url: Option<String>,
    pub created_at: chrono::NaiveDateTime,
}

/// POST /api/pay/create
#[post("/api/pay/create")]
async fn api_pay_create(
    claims: Claims,
    Component(ps): Component<PayOrderService>,
    Json(req): Json<PayCreateRequest>,
) -> Result<impl IntoResponse> {
    // 默认使用月度会员级别和微信支付
    let level = OrderLevel::Monthly; // 使用月度会员
    let pay_from = PayFrom::Wechat; // PayFrom 的默认值（微信）

    let (order_id, qrcode_url) = ps.create_order(claims.user_id, level, pay_from).await?;

    Ok(Json(PayOrderResponse {
        order_id: order_id.to_string(),
        product_name: req.product_name,
        amount: req.amount,
        status: "pending".to_string(),
        qrcode_url,
        created_at: chrono::Local::now().naive_local(),
    }))
}

/// GET /api/pay/query/{id}
#[get("/api/pay/query/{id}")]
async fn api_pay_query(
    claims: Claims,
    Path(order_id): Path<String>,
    Component(db): Component<DbConn>,
) -> Result<impl IntoResponse> {
    // 将字符串 order_id 转换为 i32
    let order_id: i32 = order_id
        .parse()
        .map_err(|_| KnownWebError::bad_request("无效的订单 ID"))?;

    let order = PayOrder::find_order_status(&db, order_id, claims.user_id)
        .await
        .context("查询订单失败")?
        .ok_or_else(|| KnownWebError::not_found("订单不存在"))?;

    Ok(Json(order))
}

