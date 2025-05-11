mod bbs;
mod home;
mod idiom;
mod paper;
mod question;

use crate::views::{user::CurrentUser, GlobalVariables};
use dtiku_base::service::system_config::SystemConfigService;
use dtiku_paper::service::exam_category::ExamCategoryService;
use spring::tracing::Level;
use spring_opentelemetry::trace;
use spring_web::{
    axum::{
        http::StatusCode,
        middleware::{self, Next},
        response::Response,
    },
    extractor::{Component, Request},
    middleware::trace::{
        DefaultMakeSpan, DefaultOnEos, DefaultOnRequest, DefaultOnResponse, TraceLayer,
    },
    Router,
};
use tower_cookies::{CookieManagerLayer, Cookies};

pub fn routers() -> Router {
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::default().level(Level::INFO))
        .on_request(DefaultOnRequest::default().level(Level::INFO))
        .on_response(DefaultOnResponse::default().level(Level::INFO))
        .on_eos(DefaultOnEos::default().level(Level::INFO));
    let http_tracing_layer = trace::HttpLayer::server(Level::INFO);
    spring_web::handler::auto_router()
        .route_layer(middleware::from_fn(with_context))
        .layer(CookieManagerLayer::new())
        .layer(trace_layer)
        .layer(http_tracing_layer)
}

async fn with_context(
    Component(ec_service): Component<ExamCategoryService>,
    Component(sc_service): Component<SystemConfigService>,
    cookies: Cookies,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let root_exam = ec_service
        .find_root_exam("gwy")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let paper_types = match root_exam {
        Some(root_exam) => ec_service
            .find_leaf_paper_types(root_exam.id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        None => vec![],
    };
    let config = sc_service
        .load_config()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let request_uri = req.uri().path().into();

    let current_user = CurrentUser {
        name: "holmofy".into(),
        avatar: "https://q1.qlogo.cn/g?b=qq&nk=1938304905@&s=100".into(),
    };
    req.extensions_mut().insert(GlobalVariables {
        user: Some(current_user),
        request_uri,
        paper_types,
        config,
        cookies,
    });
    Ok(next.run(req).await)
}
