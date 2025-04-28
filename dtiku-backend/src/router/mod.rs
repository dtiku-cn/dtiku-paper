mod config;
mod exam;
mod task;

use spring::tracing::Level;
use spring_opentelemetry::middlewares;
use spring_web::Router;

pub fn routers() -> Router {
    let http_tracing_layer = middlewares::tracing::HttpLayer::server(Level::INFO);
    spring_web::handler::auto_router().layer(http_tracing_layer)
}
