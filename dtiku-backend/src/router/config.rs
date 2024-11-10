use crate::views::{config::SystemConfig as SystemConfigView, GetListResult};
use dtiku_base::model::{enums::SystemConfigKey, system_config::Entity as SystemConfig};
use itertools::Itertools;
use spring_sea_orm::DbConn;
use spring_web::{
    axum::{response::IntoResponse, Json},
    error::Result,
    extractor::Component,
    get,
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
