mod config;
mod plugins;
mod router;

use plugins::fastembed::EmbeddingPlugin;
use spring::{auto_config, App};
use spring_web::{WebConfigurator, WebPlugin};

#[auto_config(WebConfigurator)]
#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(WebPlugin)
        .add_plugin(EmbeddingPlugin)
        .run()
        .await
}
