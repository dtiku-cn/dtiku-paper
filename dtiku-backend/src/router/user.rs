use dtiku_base::{model::UserInfo, query::UserQuery};
use sea_orm::DbConn;
use spring_sea_orm::pagination::Pagination;
use spring_web::{
    axum::{response::IntoResponse, Json},
    error::Result,
    extractor::{Component, Query},
    get,
};

#[get("/api/users")]
async fn list_users(
    Component(db): Component<DbConn>,
    Query(query): Query<UserQuery>,
    pagination: Pagination,
) -> Result<impl IntoResponse> {
    let users = UserInfo::find_page_by_query(&db, query, &pagination).await?;
    Ok(Json(users))
}

#[get("/api/user_stats")]
async fn user_stats(Component(db): Component<DbConn>) -> Result<impl IntoResponse> {
    let stats = UserInfo::stats_by_day(&db).await?;
    Ok(Json(stats))
}
