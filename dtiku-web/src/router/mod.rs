mod bbs;
mod home;
mod idiom;
mod img;
mod paper;
mod pay;
mod question;
mod shenlun_category;
mod user;

use crate::service::user::UserService;
use crate::views::{ErrorTemplate, GlobalVariables};
use askama::Template;
use axum_extra::extract::{CookieJar, Host};
use axum_extra::headers::Cookie;
use axum_extra::TypedHeader;
use derive_more::derive::Deref;
use dtiku_base::service::system_config::SystemConfigService;
use dtiku_paper::service::exam_category::ExamCategoryService;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use spring::config::env::Env;
use spring::tracing::{self, Level};
use spring_opentelemetry::trace;
use spring_web::axum::http::{request::Parts, StatusCode};
use spring_web::axum::response::{Html, IntoResponse};
use spring_web::axum::{body, RequestPartsExt};
use spring_web::error::{KnownWebError, WebError};
use spring_web::extractor::FromRequestParts;
use spring_web::{
    axum::{
        middleware::{self, Next},
        response::Response,
    },
    extractor::{Component, Request},
    middleware::trace::{
        DefaultMakeSpan, DefaultOnEos, DefaultOnFailure, DefaultOnRequest, DefaultOnResponse,
        TraceLayer,
    },
    Router,
};
use std::env;
use tokio::task_local;

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
    spring_web::handler::auto_router()
        .route_layer(middleware::from_fn(global_error_page))
        .layer(trace_layer)
        .layer(http_tracing_layer)
        .fallback(not_found_handler)
}

async fn global_error_page(
    ec_service: Component<ExamCategoryService>,
    sc_service: Component<SystemConfigService>,
    us_service: Component<UserService>,
    claims: OptionalClaims,
    host: Host,
    cookies: CookieJar,
    req: Request,
    next: Next,
) -> Response {
    let original_host = host.0.clone();
    let resp = match with_context(
        &ec_service,
        &sc_service,
        &us_service,
        &claims,
        host,
        cookies,
        req,
        next,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("request error: {e:?}");
            e.into_response()
        }
    };
    let status = resp.status();
    if status.is_client_error() || status.is_server_error() {
        let msg = resp.into_body();
        let msg = body::to_bytes(msg, usize::MAX)
            .await
            .expect("server body read failed");
        let msg = String::from_utf8(msg.to_vec()).expect("read body to string failed");
        let t = ErrorTemplate {
            status,
            msg: msg.as_str(),
            original_host: original_host.as_str(),
        };
        let html = t.render().expect("render error template failed");
        Html(html).into_response()
    } else {
        resp
    }
}

async fn not_found_handler(Host(original_host): Host) -> Response {
    let t = ErrorTemplate {
        status: StatusCode::NOT_FOUND,
        msg: "Page not found",
        original_host: original_host.as_str(),
    };
    let html = t.render().expect("render not found template failed");
    Html(html).into_response()
}

task_local! {
    pub static EXAM_ID: i16;
}

async fn with_context(
    Component(ec_service): &Component<ExamCategoryService>,
    Component(sc_service): &Component<SystemConfigService>,
    Component(us_service): &Component<UserService>,
    OptionalClaims(claims): &OptionalClaims,
    Host(original_host): Host,
    cookies: CookieJar,
    mut req: Request,
    next: Next,
) -> Result<Response, WebError> {
    let root_exam = ec_service
        .find_root_exam("gwy")
        .await
        .map_err(|e| KnownWebError::internal_server_error(format!("{e:?}")))?;
    let exam_id = match root_exam {
        Some(root_exam) => root_exam.id,
        None => 0,
    };
    let paper_types = ec_service
        .find_leaf_paper_types(exam_id)
        .await
        .map_err(|e| KnownWebError::internal_server_error(format!("{e:?}")))?;
    let config = sc_service
        .load_config()
        .await
        .map_err(|e| KnownWebError::internal_server_error(format!("{e:?}")))?;
    let request_uri = req.uri().clone();

    let current_user = match claims {
        None => None,
        Some(claims) => Some(
            us_service
                .get_user_detail(claims.user_id)
                .await
                .map_err(|e| KnownWebError::forbidden(format!("{e:?}")))?,
        ),
    };
    req.extensions_mut().insert(GlobalVariables::new(
        current_user,
        request_uri,
        original_host,
        paper_types,
        config,
        cookies,
    ));
    Ok(EXAM_ID.scope(exam_id, next.run(req)).await)
}

lazy_static! {
    static ref JWT_SECRET: String =
        env::var("JWT_SECRET").expect("JWT_SECRET not set in environment");
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: i32,
    pub exp: u64,
    pub iat: u64,
}

impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = WebError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract the token from the authorization header
        let TypedHeader(cookie) = parts
            .extract::<TypedHeader<Cookie>>()
            .await
            .map_err(|_| KnownWebError::unauthorized("invalid cookie"))?;

        let token = cookie
            .get("token")
            .ok_or_else(|| KnownWebError::unauthorized("Missing token"))?;

        // Decode the user data
        let claims = decode(token)?;

        Ok(claims)
    }
}

#[derive(Debug, Deref)]
pub struct OptionalClaims(Option<Claims>);

impl OptionalClaims {
    #[allow(unused)]
    pub fn is_none(&self) -> bool {
        self.0.is_none()
    }
}

impl<S> FromRequestParts<S> for OptionalClaims
where
    S: Send + Sync,
{
    type Rejection = WebError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(cookie) = parts
            .extract::<TypedHeader<Cookie>>()
            .await
            .map_err(|_| KnownWebError::unauthorized("invalid cookie"))?;

        // Decode the user data
        let claims = match cookie.get("token") {
            Some(token) => {
                if token.is_empty() {
                    None
                } else {
                    decode(token).ok()
                }
            }
            None => None,
        };

        Ok(Self(claims))
    }
}

/// JWT encode
#[allow(unused)]
pub fn encode(claims: Claims) -> anyhow::Result<String> {
    let header = Header::new(Algorithm::HS256);
    let encode_key = EncodingKey::from_secret(JWT_SECRET.as_bytes());
    let token = jsonwebtoken::encode::<Claims>(&header, &claims, &encode_key)
        .map_err(|_| KnownWebError::internal_server_error("Token created error"))?;

    Ok(token)
}

/// JWT decode
pub fn decode(token: &str) -> anyhow::Result<Claims> {
    let validation = Validation::new(Algorithm::HS256);
    let decode_key = DecodingKey::from_secret(JWT_SECRET.as_bytes());
    let token_data =
        jsonwebtoken::decode::<Claims>(&token, &decode_key, &validation).map_err(|e| {
            tracing::error!("{:?}", e);
            KnownWebError::unauthorized(format!("invalid token:{token}"))
        })?;
    Ok(token_data.claims)
}
