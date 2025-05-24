mod bbs;
mod home;
mod idiom;
mod paper;
mod question;
mod shenlun_category;
mod user;

use crate::service::user::UserService;
use crate::views::GlobalVariables;
use axum_extra::extract::CookieJar;
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
use spring_web::axum::http::header;
use spring_web::axum::http::request::Parts;
use spring_web::axum::RequestPartsExt;
use spring_web::error::{KnownWebError, WebError};
use spring_web::extractor::FromRequestParts;
use spring_web::{
    axum::{
        http::StatusCode,
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
    let trace_layer = match Env::init() {
        Env::Dev => {
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().level(Level::INFO))
                .on_request(DefaultOnRequest::default().level(Level::INFO))
                .on_response(DefaultOnResponse::default().level(Level::INFO))
                .on_failure(DefaultOnFailure::default().level(Level::INFO))
                .on_eos(DefaultOnEos::default());
        }
        _ => {
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().level(Level::INFO))
                .on_request(DefaultOnRequest::default())
                .on_response(DefaultOnResponse::default())
                .on_failure(DefaultOnFailure::default())
                .on_eos(DefaultOnEos::default());
        }
    };
    let http_tracing_layer = trace::HttpLayer::server(Level::INFO);
    spring_web::handler::auto_router()
        .route_layer(middleware::from_fn(with_context))
        .layer(trace_layer)
        .layer(http_tracing_layer)
}

task_local! {
    pub static EXAM_ID: i16;
}

async fn with_context(
    Component(ec_service): Component<ExamCategoryService>,
    Component(sc_service): Component<SystemConfigService>,
    Component(us_service): Component<UserService>,
    OriginalHost(original_host): OriginalHost,
    OptionalClaims(claims): OptionalClaims,
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
                    Some(decode(token)?)
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
            KnownWebError::unauthorized("invalid token")
        })?;
    Ok(token_data.claims)
}

#[derive(Debug, Deref)]
pub struct OriginalHost(String);

impl<S> FromRequestParts<S> for OriginalHost
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let host = parts
            .headers
            .get("x-forwarded-host")
            .or_else(|| parts.headers.get(header::HOST))
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown".into());

        Ok(OriginalHost(host))
    }
}
