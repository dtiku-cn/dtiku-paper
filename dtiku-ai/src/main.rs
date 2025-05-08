mod config;
mod plugins;
mod service;

use plugins::fastembed::EmbeddingPlugin;
use spring::App;
use spring_grpc::GrpcPlugin;

#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(EmbeddingPlugin)
        .add_plugin(GrpcPlugin)
        .run()
        .await
}
