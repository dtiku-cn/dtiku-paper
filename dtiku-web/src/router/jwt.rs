use crate::router::error_messages;
use axum_extra::headers::Cookie;
use axum_extra::TypedHeader;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use spring::tracing;
use spring_web::axum::http::request::Parts;
use spring_web::axum::RequestPartsExt;
use spring_web::error::{KnownWebError, WebError};
use spring_web::extractor::FromRequestParts;
use std::env;
use std::sync::OnceLock;

static JWT_SECRET: OnceLock<String> = OnceLock::new();

/// 获取 JWT 密钥
fn get_jwt_secret() -> &'static str {
    JWT_SECRET.get_or_init(|| env::var("JWT_SECRET").expect("JWT_SECRET not set in environment"))
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
            .map_err(|_| KnownWebError::unauthorized(error_messages::INVALID_COOKIE))?;

        let token = cookie
            .get("token")
            .ok_or_else(|| KnownWebError::unauthorized(error_messages::MISSING_TOKEN))?;

        // Decode the user data
        let claims = decode(token)?;

        Ok(claims)
    }
}

#[derive(Debug, derive_more::derive::Deref)]
pub struct OptionalClaims(pub Option<Claims>);

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
            .map_err(|_| KnownWebError::unauthorized(error_messages::INVALID_COOKIE))?;

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
    let encode_key = EncodingKey::from_secret(get_jwt_secret().as_bytes());
    let token = jsonwebtoken::encode::<Claims>(&header, &claims, &encode_key)
        .map_err(|_| KnownWebError::internal_server_error(error_messages::TOKEN_CREATION_ERROR))?;

    Ok(token)
}

/// JWT decode
pub fn decode(token: &str) -> anyhow::Result<Claims> {
    let validation = Validation::new(Algorithm::HS256);
    let decode_key = DecodingKey::from_secret(get_jwt_secret().as_bytes());
    let token_data =
        jsonwebtoken::decode::<Claims>(&token, &decode_key, &validation).map_err(|e| {
            tracing::error!("{:?}", e);
            KnownWebError::unauthorized(error_messages::invalid_token_msg(token))
        })?;
    Ok(token_data.claims)
}
