use crate::views::{config::SystemConfig as SystemConfigView, GetListResult};
use anyhow::Context;
use dtiku_base::model::{
    enums::SystemConfigKey,
    system_config::{self, Entity as SystemConfig},
};
use itertools::Itertools;
use sea_orm::ActiveModelTrait;
use sea_orm::Set;
use spring_sea_orm::DbConn;
use spring_web::{
    axum::{response::IntoResponse, Json},
    error::Result,
    extractor::{Component, Path},
    get, put,
};
use std::collections::HashMap;
use strum::IntoEnumIterator;

#[get("/api/configs")]
async fn list_all_config(Component(db): Component<DbConn>) -> Result<impl IntoResponse> {
    let configs = SystemConfig::find_all(&db).await?;
    let mut key_map: HashMap<_, _> = configs.into_iter().map(|c| (c.key, c)).collect();
    let result: Vec<SystemConfigView> = SystemConfigKey::iter()
        .map(|key| {
            key_map
                .remove(&key)
                .map(|m| m.into())
                .unwrap_or_else(|| key.into())
        })
        .collect_vec();
    Ok(Json(GetListResult::from(result)))
}

#[put("/api/configs/:key")]
async fn save_config(
    Component(db): Component<DbConn>,
    Path(key): Path<SystemConfigKey>,
    Json(value): Json<serde_json::Value>,
) -> Result<impl IntoResponse> {
    let model = SystemConfig::find_by_key(&db, key).await?;
    let active_model = match model {
        Some(m) => system_config::ActiveModel {
            id: Set(m.id),
            key: Set(key),
            value: Set(value),
            ..Default::default()
        },
        None => system_config::ActiveModel {
            key: Set(key),
            value: Set(value),
            ..Default::default()
        },
    };
    active_model.save(&db).await.context("save config failed")?;
    Ok(Json("success"))
}
