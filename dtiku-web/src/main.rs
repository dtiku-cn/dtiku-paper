pub mod data;
mod router;

use spring::{auto_config, App};
use spring_sea_orm::SeaOrmPlugin;
use spring_web::{WebConfigurator, WebPlugin};

#[auto_config(WebConfigurator)]
#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(WebPlugin)
        .add_plugin(SeaOrmPlugin)
        .run()
        .await
}
