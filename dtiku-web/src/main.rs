mod plugins;
mod query;
mod router;
mod rpc;
mod service;
mod views;

use dtiku_pay::PayPlugin;
use plugins::grpc_client::GrpcClientPlugin;
use spring::App;
use spring_opentelemetry::{
    KeyValue, OpenTelemetryPlugin, ResourceConfigurator, SERVICE_NAME, SERVICE_VERSION,
};
use spring_redis::RedisPlugin;
use spring_sea_orm::SeaOrmPlugin;
use spring_stream::StreamPlugin;
use spring_web::{WebConfigurator, WebPlugin};

#[tokio::main]
async fn main() {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .ok();
    App::new()
        .opentelemetry_attrs([
            KeyValue::new(SERVICE_NAME, env!("CARGO_PKG_NAME")),
            KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
        ])
        .add_router(router::routers())
        .add_plugin(WebPlugin)
        .add_plugin(RedisPlugin)
        .add_plugin(SeaOrmPlugin)
        .add_plugin(StreamPlugin)
        .add_plugin(OpenTelemetryPlugin)
        .add_plugin(GrpcClientPlugin)
        .add_plugin(PayPlugin)
        .run()
        .await
}
