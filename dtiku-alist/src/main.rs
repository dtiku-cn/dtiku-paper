mod plugins;
mod router;
mod rpc;

use spring::{auto_config, App};
use spring_opendal::OpenDALPlugin;
use spring_opentelemetry::{
    KeyValue, OpenTelemetryPlugin, ResourceConfigurator, SERVICE_NAME, SERVICE_VERSION,
};
use spring_sea_orm::SeaOrmPlugin;
use spring_stream::StreamPlugin;
use spring_web::{WebConfigurator, WebPlugin};

#[auto_config(WebConfigurator)]
#[tokio::main]
async fn main() {
    App::new()
        .opentelemetry_attrs([
            KeyValue::new(SERVICE_NAME, env!("CARGO_PKG_NAME")),
            KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
        ])
        .add_plugin(WebPlugin)
        .add_plugin(StreamPlugin)
        .add_plugin(SeaOrmPlugin)
        .add_plugin(OpenTelemetryPlugin)
        .add_plugin(OpenDALPlugin)
        .run()
        .await
}
