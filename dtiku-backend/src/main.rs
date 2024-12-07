mod config;
mod jobs;
mod plugins;
mod router;
mod utils;
mod views;

use chrono::Local;
use dtiku_base::model::schedule_task;
use plugins::fastembed::EmbeddingPlugin;
use spring::{auto_config, App};
use spring_redis::RedisPlugin;
use spring_sea_orm::SeaOrmPlugin;
use spring_sqlx::SqlxPlugin;
use spring_stream::{extractor::Json, StreamPlugin};
use spring_web::{WebConfigurator, WebPlugin};

// #[auto_config(WebConfigurator)]
#[tokio::main]
async fn main() {
    let app = App::new()
        .add_plugin(WebPlugin)
        .add_plugin(SeaOrmPlugin)
        .add_plugin(SqlxPlugin)
        .add_plugin(StreamPlugin)
        .add_plugin(RedisPlugin)
        .add_plugin(EmbeddingPlugin)
        .build()
        .await;

    if let Ok(app) = app {
        let task = schedule_task::Model {
            id: 1,
            version: 1,
            ty: dtiku_base::model::enums::ScheduleTaskType::FenbiSync,
            active: true,
            context: serde_json::Value::Null,
            error_count: 0,
            error_cause: "".to_string(),
            created: Local::now().naive_local(),
            modified: Local::now().naive_local(),
        };
        jobs::refresh_cache(Json(task)).await;
    }
}
