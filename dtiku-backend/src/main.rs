mod job;
mod router;
mod views;

use spring::{auto_config, App};
use spring_redis::RedisPlugin;
use spring_sea_orm::SeaOrmPlugin;
use spring_sqlx::SqlxPlugin;
use spring_stream::StreamPlugin;
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
        .run()
        .await
}
