mod bbs;
mod error_messages;
mod home;
mod idiom;
mod jwt;
mod key_point;
mod middleware;
mod paper;
mod pay;
mod question;
mod shenlun_category;
mod user;

pub use jwt::{decode, Claims};
pub use middleware::EXAM_ID;

use axum_client_ip::ClientIpSource;
use spring::config::env::Env;
use spring::tracing::{self, Level};
use spring_opentelemetry::trace;
use spring_web::{
    axum::middleware as axum_middleware,
    middleware::trace::{
        DefaultMakeSpan, DefaultOnEos, DefaultOnFailure, DefaultOnRequest, DefaultOnResponse,
        TraceLayer,
    },
    Router,
};
use std::sync::Arc;
use std::time::Duration;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::SmartIpKeyExtractor;
use tower_governor::GovernorLayer;

pub fn routers() -> Router {
    let env = Env::init();
    let trace_layer = match env {
        Env::Dev => TraceLayer::new_for_http()
            .make_span_with(DefaultMakeSpan::default().level(Level::INFO))
            .on_request(DefaultOnRequest::default().level(Level::INFO))
            .on_response(DefaultOnResponse::default().level(Level::INFO))
            .on_failure(DefaultOnFailure::default().level(Level::INFO))
            .on_eos(DefaultOnEos::default()),
        _ => TraceLayer::new_for_http()
            .make_span_with(DefaultMakeSpan::default().level(Level::INFO))
            .on_request(DefaultOnRequest::default())
            .on_response(DefaultOnResponse::default())
            .on_failure(DefaultOnFailure::default())
            .on_eos(DefaultOnEos::default()),
    };

    let http_tracing_layer = trace::HttpLayer::server(Level::INFO);

    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(1) // 允许的平均请求速率
            .burst_size(20) // 允许突发的最大请求数
            // 优先从请求头里取 X-Forwarded-For、Forwarded 等常见代理头，取不到再回退到对端 IP
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .unwrap(),
    );

    let governor_limiter = governor_conf.limiter().clone();
    let interval = Duration::from_secs(60);
    // 单独的后台线程定时清理hashmap中的key，防止内存泄漏
    std::thread::spawn(move || loop {
        std::thread::sleep(interval);
        tracing::debug!("rate limiting storage size: {}", governor_limiter.len());
        governor_limiter.retain_recent();
    });

    spring_web::handler::auto_router()
        .route_layer(axum_middleware::from_fn(middleware::with_global_context))
        .route_layer(axum_middleware::from_fn(middleware::global_error_page))
        .route_layer(axum_middleware::from_fn(middleware::anti_bot))
        .layer(trace_layer)
        .layer(http_tracing_layer)
        .layer(match env {
            Env::Dev => ClientIpSource::ConnectInfo.into_extension(),
            _ => ClientIpSource::RightmostXForwardedFor.into_extension(),
        })
        .layer(GovernorLayer::new(governor_conf))
        .fallback(middleware::not_found_handler)
}
