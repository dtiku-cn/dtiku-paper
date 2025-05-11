mod router;
mod views;
mod query;

use spring::App;
use spring_opentelemetry::{
    KeyValue, OpenTelemetryPlugin, ResourceConfigurator, SERVICE_NAME, SERVICE_VERSION,
};
use spring_redis::RedisPlugin;
use spring_sea_orm::SeaOrmPlugin;
use spring_web::{WebConfigurator, WebPlugin};

#[tokio::main]
async fn main() {
    App::new()
        .opentelemetry_attrs([
            KeyValue::new(SERVICE_NAME, env!("CARGO_PKG_NAME")),
            KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
        ])
        .add_router(router::routers())
        .add_plugin(WebPlugin)
        .add_plugin(RedisPlugin)
        .add_plugin(SeaOrmPlugin)
        // .add_plugin(OpenTelemetryPlugin)
        .run()
        .await
}
