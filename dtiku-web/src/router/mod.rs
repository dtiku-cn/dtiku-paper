mod bbs;
mod home;
mod idiom;
mod paper;
mod question;

use spring::tracing::Level;
use spring_opentelemetry::trace;
use spring_web::{
    middleware::trace::{
        DefaultMakeSpan, DefaultOnEos, DefaultOnRequest, DefaultOnResponse, TraceLayer,
    },
    Router,
};

pub fn routers() -> Router {
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::default().level(Level::INFO))
        .on_request(DefaultOnRequest::default().level(Level::INFO))
        .on_response(DefaultOnResponse::default().level(Level::INFO))
        .on_eos(DefaultOnEos::default().level(Level::INFO));
    let http_tracing_layer = trace::HttpLayer::server(Level::INFO);
    spring_web::handler::auto_router()
        .layer(trace_layer)
        .layer(http_tracing_layer)
}
