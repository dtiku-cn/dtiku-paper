use anyhow::Context;
use chrono::NaiveDate;
use dtiku_base::{model::UserInfo, query::UserQuery};
use sea_orm::DbConn;
use serde::{Deserialize, Serialize};
use spring_redis::Redis;
use spring_sea_orm::pagination::Pagination;
use spring_web::{
    axum::{response::IntoResponse, Json},
    error::Result,
    extractor::{Component, Query},
    get,
};

#[derive(Debug, Serialize)]
pub struct OnlineUser {
    pub id: i32,
    pub name: String,
    pub avatar: String,
    pub modified: String,
}

#[derive(Debug, Serialize)]
pub struct OnlineUserStats {
    pub online_count: usize,
    pub online_users: Vec<OnlineUser>,
}

#[get("/api/users")]
async fn list_users(
    Component(db): Component<DbConn>,
    Query(query): Query<UserQuery>,
    pagination: Pagination,
) -> Result<impl IntoResponse> {
    let users = UserInfo::find_page_by_query(&db, query, &pagination).await?;
    Ok(Json(users))
}

#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
}

#[get("/api/user_stats")]
async fn user_stats(
    Component(db): Component<DbConn>,
    Query(query): Query<StatsQuery>,
) -> Result<impl IntoResponse> {
    let stats = UserInfo::stats_by_day(&db, query.start_date, query.end_date).await?;
    Ok(Json(stats))
}

#[get("/api/online_users")]
async fn online_users(
    Component(db): Component<DbConn>,
    Component(mut redis): Component<Redis>,
) -> Result<impl IntoResponse> {
    // 使用 SCAN 命令迭代所有 user:* 的 key，避免阻塞 Redis
    const MAX_USERS: usize = 100; // 限制最大返回用户数
    let mut user_ids = Vec::new();
    let mut cursor = 0u64;

    // SCAN 循环，直到遍历完成或达到用户上限
    loop {
        // SCAN cursor MATCH user:* COUNT 50
        let (new_cursor, keys): (u64, Vec<String>) = spring_redis::redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg("user:*")
            .arg("COUNT")
            .arg(50)
            .query_async(&mut redis)
            .await
            .context("Redis SCAN failed")?;

        // 提取用户ID
        for key in keys {
            if user_ids.len() >= MAX_USERS {
                break;
            }
            if let Some(id_str) = key.strip_prefix("user:") {
                if let Ok(user_id) = id_str.parse::<i32>() {
                    user_ids.push(user_id);
                }
            }
        }

        cursor = new_cursor;
        
        // cursor == 0 表示遍历完成，或者已经达到用户数量上限
        if cursor == 0 || user_ids.len() >= MAX_USERS {
            break;
        }
    }

    // 批量查询用户信息，避免 N+1 问题
    let users = if user_ids.is_empty() {
        Vec::new()
    } else {
        UserInfo::find_user_by_ids(&db, user_ids)
            .await
            .context("Query user info failed")?
    };

    // 转换为响应格式
    let online_users: Vec<OnlineUser> = users
        .into_iter()
        .map(|user| OnlineUser {
            id: user.id,
            name: user.name,
            avatar: user.avatar,
            modified: user.modified.format("%Y-%m-%d %H:%M:%S").to_string(),
        })
        .collect();

    let stats = OnlineUserStats {
        online_count: online_users.len(),
        online_users,
    };

    Ok(Json(stats))
}
