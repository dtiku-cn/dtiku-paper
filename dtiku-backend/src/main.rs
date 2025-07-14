mod config;
mod jobs;
mod plugins;
mod router;
mod service;
mod utils;
mod views;

use plugins::{embedding::EmbeddingPlugin, jobs::RunningJobsPlugin};
use spring::{auto_config, App};
use spring_opentelemetry::{
    KeyValue, OpenTelemetryPlugin, ResourceConfigurator, SERVICE_NAME, SERVICE_VERSION,
};
use spring_redis::RedisPlugin;
use spring_sea_orm::SeaOrmPlugin;
use spring_sqlx::SqlxPlugin;
use spring_stream::{StreamConfigurator, StreamPlugin};
use spring_web::{WebConfigurator, WebPlugin};

#[auto_config(StreamConfigurator)]
#[tokio::main]
async fn main() {
    App::new()
        .opentelemetry_attrs([
            KeyValue::new(SERVICE_NAME, env!("CARGO_PKG_NAME")),
            KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
        ])
        .add_router(router::routers())
        .add_consumer(jobs::consumer())
        .add_plugin(WebPlugin)
        .add_plugin(SeaOrmPlugin)
        .add_plugin(SqlxPlugin)
        .add_plugin(StreamPlugin)
        .add_plugin(RedisPlugin)
        .add_plugin(RunningJobsPlugin)
        .add_plugin(OpenTelemetryPlugin)
        .add_plugin(EmbeddingPlugin)
        .run()
        .await
}
