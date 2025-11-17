use anyhow::Context;
use chrono::NaiveDate;
use dtiku_base::model::UserInfo;
use dtiku_pay::model::pay_order::{Column, Entity as PayOrder};
use sea_orm::{ColumnTrait, DbConn, EntityTrait, Order, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use spring_sea_orm::pagination::{Page, Pagination, PaginationExt};
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

#[derive(Debug, Serialize)]
pub struct PayOrderWithUser {
    #[serde(flatten)]
    pub order: dtiku_pay::model::pay_order::Model,
    pub user_name: Option<String>,
    pub user_avatar: Option<String>,
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

    // 提取所有用户ID
    let user_ids: Vec<i32> = page_result
        .content
        .iter()
        .map(|order| order.user_id)
        .collect();

    // 批量查询用户信息
    let users = if !user_ids.is_empty() {
        UserInfo::find_user_by_ids(&db, user_ids)
            .await
            .unwrap_or_default()
    } else {
        vec![]
    };

    // 构建用户ID到用户信息的映射
    let user_map: std::collections::HashMap<i32, (String, String)> = users
        .into_iter()
        .map(|user| (user.id, (user.name, user.avatar)))
        .collect();

    // 组装结果
    let orders_with_user: Vec<PayOrderWithUser> = page_result
        .content
        .into_iter()
        .map(|order| {
            let (user_name, user_avatar) = user_map
                .get(&order.user_id)
                .map(|(name, avatar)| (Some(name.clone()), Some(avatar.clone())))
                .unwrap_or((None, None));
            
            PayOrderWithUser {
                user_name,
                user_avatar,
                order,
            }
        })
        .collect();

    let result = Page {
        content: orders_with_user,
        size: page_result.size,
        page: page_result.page,
        total_elements: page_result.total_elements,
        total_pages: page_result.total_pages,
    };

    Ok(Json(result))
}

#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
}

#[get("/api/pay/stats")]
async fn pay_stats(
    Component(db): Component<DbConn>,
    Query(query): Query<StatsQuery>,
) -> Result<impl IntoResponse> {
    let stats = PayOrder::stats_by_day(&db, query.start_date, query.end_date).await?;
    Ok(Json(stats))
}

