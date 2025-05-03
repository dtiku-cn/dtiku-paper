mod config;
mod exam;
mod task;

use spring::tracing::Level;
use spring_opentelemetry::trace;
use spring_web::Router;

pub fn routers() -> Router {
    let http_tracing_layer = trace::HttpLayer::server(Level::INFO);
    spring_web::handler::auto_router().layer(http_tracing_layer)
}
