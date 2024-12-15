mod config;
mod jobs;
mod plugins;
mod router;
mod utils;
mod views;

use plugins::{fastembed::EmbeddingPlugin, jobs::RunningJobsPlugin};
use spring::{auto_config, App};
use spring_redis::RedisPlugin;
use spring_sea_orm::SeaOrmPlugin;
use spring_sqlx::SqlxPlugin;
use spring_stream::{StreamConfigurator, StreamPlugin};
use spring_web::{WebConfigurator, WebPlugin};

#[auto_config(WebConfigurator)]
#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(WebPlugin)
        .add_plugin(SeaOrmPlugin)
        .add_plugin(SqlxPlugin)
        .add_plugin(StreamPlugin)
        .add_plugin(RedisPlugin)
        .add_plugin(EmbeddingPlugin)
        .add_plugin(RunningJobsPlugin)
        .add_consumer(jobs::consumer())
        .run()
        .await
}
