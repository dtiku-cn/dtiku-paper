use crate::router::Claims;
use anyhow::Context;
use dtiku_pay::{
    model::{OrderLevel, OrderStatus, PayFrom, PayOrder},
    service::pay_order::PayOrderService,
};
use schemars::JsonSchema;
use sea_orm::DbConn;
use serde::{Deserialize, Serialize};
use spring_web::{
    axum::Json,
    error::{KnownWebError, Result},
    extractor::{Component, Path},
    get_api, post_api,
};
#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct PayCreateRequest {
    pub product_name: String,
    pub product_description: Option<String>,
    pub amount: i64,
    pub payment_method: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct PayOrderResponse {
    pub order_id: String,
    pub product_name: String,
    pub amount: i64,
    pub status: String,
    pub qrcode_url: Option<String>,
    pub created_at: chrono::NaiveDateTime,
}

/// POST /api/pay/create
#[post_api("/api/pay/create")]
async fn api_pay_create(
    claims: Claims,
    Component(ps): Component<PayOrderService>,
    Json(req): Json<PayCreateRequest>,
) -> Result<Json<PayOrderResponse>> {
    let level = OrderLevel::Monthly;
    let pay_from = PayFrom::Wechat;

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
#[get_api("/api/pay/query/{id}")]
async fn api_pay_query(
    claims: Claims,
    Path(order_id): Path<String>,
    Component(db): Component<DbConn>,
) -> Result<Json<OrderStatus>> {
    let order_id: i32 = order_id
        .parse()
        .map_err(|_| KnownWebError::bad_request("无效的订单 ID"))?;

    let order = PayOrder::find_order_status(&db, order_id, claims.user_id)
        .await
        .context("查询订单失败")?
        .ok_or_else(|| KnownWebError::not_found("订单不存在"))?;

    Ok(Json(order))
}
