use crate::views::{config::SystemConfig as SystemConfigView, GetListResult};
use anyhow::Context;
use dtiku_base::model::{
    enums::SystemConfigKey,
    system_config::{self, Entity as SystemConfig},
};
use itertools::Itertools;
use sea_orm::{
    ActiveModelTrait, ConnectionTrait, DbBackend, ExecResult, FromQueryResult, Statement,
};
use sea_orm::{DatabaseBackend, Set};
use serde::Serialize;
use spring_sea_orm::DbConn;
use spring_web::{
    axum::{response::IntoResponse, Json},
    error::Result,
    extractor::{Component, Path},
    get, post, put,
};
use std::collections::HashMap;
use strum::IntoEnumIterator;

#[derive(Debug, Serialize, FromQueryResult)]
pub struct MaterializedView {
    matviewname: String,
    schemaname: String,
    ispopulated: bool,
}

#[get("/api/matviews")]
async fn list_all_matviews(Component(db): Component<DbConn>) -> Result<impl IntoResponse> {
    Ok(Json(
        MaterializedView::find_by_statement(Statement::from_string(
            DbBackend::Postgres,
            r#"
        SELECT schemaname,
        matviewname,
        ispopulated
        FROM pg_matviews
        "#,
        ))
        .all(&db)
        .await
        .context("fetch pg_matviews failed")?,
    ))
}

#[post("/api/matviews/{name}/refresh")]
async fn refresh_config(
    Component(db): Component<DbConn>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse> {
    let result: ExecResult = db
        .execute(Statement::from_string(
            DatabaseBackend::Postgres,
            format!("refresh materialized view {name}"),
        ))
        .await
        .context("refresh materialzed view failed")?;
    Ok(format!("success: {}", result.rows_affected()))
}

#[get("/api/matviews/{name}/definition")]
async fn save_config(
    Component(db): Component<DbConn>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse> {
    let definition: Option<String> = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"
            SELECT definition
            FROM pg_matviews
            WHERE matviewname = $1
            "#,
            vec![name.into()],
        ))
        .await
        .context("fetch pg_matviews definition failed")?
        .map(|r| r.try_get_by_index(0).unwrap());
    Ok(definition.unwrap_or_default())
}
