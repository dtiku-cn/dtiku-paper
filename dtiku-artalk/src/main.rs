mod service;

use spring::App;
use spring_grpc::GrpcPlugin;
use spring_sqlx::SqlxPlugin;

#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(SqlxPlugin)
        .add_plugin(GrpcPlugin)
        .run()
        .await
}
