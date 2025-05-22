mod bbs;
mod home;
mod idiom;
mod paper;
mod question;
mod shenlun_category;

use crate::views::{user::CurrentUser, GlobalVariables};
use axum_extra::extract::CookieJar;
use axum_extra::headers::Cookie;
use axum_extra::TypedHeader;
use dtiku_base::service::system_config::SystemConfigService;
use dtiku_paper::service::exam_category::ExamCategoryService;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use spring::tracing::{self, Level};
use spring_opentelemetry::trace;
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
use std::ops::Deref;
use tokio::task_local;

pub fn routers() -> Router {
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::default())
        .on_request(DefaultOnRequest::default())
        .on_response(DefaultOnResponse::default())
        .on_failure(DefaultOnFailure::default())
        .on_eos(DefaultOnEos::default());
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
    cookies: CookieJar,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let root_exam = ec_service
        .find_root_exam("gwy")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let exam_id = match root_exam {
        Some(root_exam) => root_exam.id,
        None => 0,
    };
    let paper_types = ec_service
        .find_leaf_paper_types(exam_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let config = sc_service
        .load_config()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let request_uri = req.uri().path().into();

    let has_user = req
        .uri()
        .query()
        .map(|query| query.contains("user"))
        .unwrap_or_default();
    let current_user = if has_user {
        Some(CurrentUser {
            name: "holmofy".into(),
            avatar: "https://q1.qlogo.cn/g?b=qq&nk=1938304905@&s=100".into(),
        })
    } else {
        None
    };
    req.extensions_mut().insert(GlobalVariables::new(
        current_user,
        request_uri,
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

pub struct OptionalClaims(Option<Claims>);

impl OptionalClaims {
    pub fn is_none(&self) -> bool {
        self.0.is_none()
    }
}

impl Deref for OptionalClaims {
    type Target = Option<Claims>;

    fn deref(&self) -> &Self::Target {
        &self.0
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
            Some(token) => Some(decode(token)?),
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
