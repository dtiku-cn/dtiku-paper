mod idiom;
mod issue;
mod paper;
mod pay;
mod question;
mod system;
mod user;

use axum_client_ip::ClientIpSource;
use axum_extra::headers::Cookie;
use axum_extra::TypedHeader;
use derive_more::derive::Deref;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use spring::tracing::{self, Level};
use spring_opentelemetry::trace;
use spring_web::axum::http::{request::Parts, StatusCode};
use spring_web::axum::response::{IntoResponse, Response};
use spring_web::axum::{
    body,
    middleware::{self, Next},
    RequestPartsExt,
};
use spring_web::error::{KnownWebError, WebError};
use spring_web::extractor::FromRequestParts;
use spring_web::{
    extractor::Request,
    middleware::trace::{
        DefaultMakeSpan, DefaultOnEos, DefaultOnFailure, DefaultOnRequest, DefaultOnResponse,
        TraceLayer,
    },
    Router,
};
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::SmartIpKeyExtractor;
use tower_governor::GovernorLayer;

pub fn routers() -> Router {
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::default().level(Level::INFO))
        .on_request(DefaultOnRequest::default().level(Level::INFO))
        .on_response(DefaultOnResponse::default().level(Level::INFO))
        .on_failure(DefaultOnFailure::default().level(Level::INFO))
        .on_eos(DefaultOnEos::default());

    let http_tracing_layer = trace::HttpLayer::server(Level::INFO);

    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(10) // 移动端API更宽松的限流
            .burst_size(30)
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .unwrap(),
    );

    let governor_limiter = governor_conf.limiter().clone();
    let interval = Duration::from_secs(60);
    std::thread::spawn(move || loop {
        std::thread::sleep(interval);
        tracing::debug!("rate limiting storage size: {}", governor_limiter.len());
        governor_limiter.retain_recent();
    });

    spring_web::handler::auto_router()
        .route_layer(middleware::from_fn(error_handler))
        .layer(trace_layer)
        .layer(http_tracing_layer)
        .layer(ClientIpSource::RightmostXForwardedFor.into_extension())
        .layer(GovernorLayer::new(governor_conf))
        .fallback(not_found_handler)
}

async fn error_handler(req: Request, next: Next) -> Response {
    let resp = next.run(req).await;
    let status = resp.status();

    if status.is_client_error() || status.is_server_error() {
        let msg = resp.into_body();
        let msg = body::to_bytes(msg, usize::MAX)
            .await
            .expect("server body read failed");
        let msg = String::from_utf8(msg.to_vec()).expect("read body to string failed");

        let error_json = serde_json::json!({
            "error": true,
            "status": status.as_u16(),
            "message": msg,
        });

        (status, axum::Json(error_json)).into_response()
    } else {
        resp
    }
}

async fn not_found_handler() -> Response {
    (
        StatusCode::NOT_FOUND,
        axum::Json(serde_json::json!({
            "error": true,
            "status": 404,
            "message": "API endpoint not found"
        })),
    )
        .into_response()
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
