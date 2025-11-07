use anyhow::Context;
use dtiku_pay::model::pay_order::{Column, Entity as PayOrder};
use sea_orm::{ColumnTrait, DbConn, EntityTrait, Order, QueryFilter, QueryOrder};
use serde::Deserialize;
use spring_sea_orm::pagination::{Pagination, PaginationExt};
use spring_web::{
    axum::{response::IntoResponse, Json},
    error::Result,
    extractor::{Component, Query},
    get,
};

#[derive(Debug, Deserialize)]
pub struct PayOrderQuery {
    pub user_id: Option<i32>,
    pub status: Option<String>,
    pub pay_from: Option<String>,
}

#[get("/api/pay/orders")]
async fn list_pay_orders(
    Component(db): Component<DbConn>,
    Query(query): Query<PayOrderQuery>,
    pagination: Pagination,
) -> Result<impl IntoResponse> {
    let mut select = PayOrder::find();

    // 应用筛选条件
    if let Some(user_id) = query.user_id {
        select = select.filter(Column::UserId.eq(user_id));
    }
    if let Some(status) = query.status {
        select = select.filter(Column::Status.eq(status));
    }
    if let Some(pay_from) = query.pay_from {
        select = select.filter(Column::PayFrom.eq(pay_from));
    }

    // 按创建时间倒序排列
    select = select.order_by(Column::Created, Order::Desc);

    // 分页查询
    let page_result = select
        .page(&db, &pagination)
        .await
        .context("查询支付订单失败")?;

    Ok(Json(page_result))
}

