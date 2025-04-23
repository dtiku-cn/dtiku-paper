mod config;
mod plugins;
mod service;

use plugins::fastembed::EmbeddingPlugin;
use spring::App;

#[tokio::main]
async fn main() {
    App::new().add_plugin(EmbeddingPlugin).run().await
}
