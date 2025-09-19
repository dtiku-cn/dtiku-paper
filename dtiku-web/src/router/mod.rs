mod bbs;
mod home;
mod idiom;
mod img;
mod key_point;
mod paper;
mod pay;
mod question;
mod shenlun_category;
mod traffic;
mod user;

use crate::service::user::UserService;
use crate::views::{AntiBotTemplate, ErrorTemplate, GlobalVariables};
use askama::Template;
use axum_client_ip::{ClientIp, ClientIpSource};
use axum_extra::extract::{CookieJar, Host};
use axum_extra::headers::{Cookie, UserAgent};
use axum_extra::TypedHeader;
use chrono::Utc;
use derive_more::derive::Deref;
use dtiku_base::service::system_config::SystemConfigService;
use dtiku_paper::service::exam_category::ExamCategoryService;
use http::HeaderValue;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::task_local;
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
            .burst_size(10) // 允许突发的最大请求数
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
        .route_layer(middleware::from_fn(global_error_page))
        .layer(trace_layer)
        .layer(http_tracing_layer)
        .layer(match env {
            Env::Dev => ClientIpSource::ConnectInfo.into_extension(),
            _ => ClientIpSource::RightmostXForwardedFor.into_extension(),
        })
        .layer(GovernorLayer::new(governor_conf))
        .fallback(not_found_handler)
}

async fn global_error_page(
    ec_service: Component<ExamCategoryService>,
    sc_service: Component<SystemConfigService>,
    us_service: Component<UserService>,
    ClientIp(client_ip): ClientIp,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    claims: OptionalClaims,
    host: Host,
    cookies: CookieJar,
    req: Request,
    next: Next,
) -> Response {
    if !req.uri().path().starts_with("/api") && !req.uri().path().starts_with("/pay") {
        if let Some(resp) = anti_bot(&sc_service, &cookies, user_agent, client_ip).await {
            return resp;
        }
    }
    let fp = cookies
        .get("x-fp")
        .map(|x_fp_id| x_fp_id.value())
        .map(|x_fp_id| format!("fp:{x_fp_id}"))
        .unwrap_or("".to_string());
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
        Ok(mut r) => {
            let remote_user = match &*claims {
                Some(c) => format!("u:{}", c.user_id),
                None => fp,
            };
            if let Ok(remote_user) = HeaderValue::from_str(&remote_user) {
                r.headers_mut().insert("X-Remote-User", remote_user);
            }
            r
        }
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

const GOOGLE_DOMAINS: [&str; 2] = ["googlebot.com", "google.com"];
const BING_DOMAINS: [&str; 1] = ["search.msn.com"];
const BAIDU_DOMAINS: [&str; 1] = ["baidu.com"];
const SOGOU_DOMAINS: [&str; 1] = ["sogou.com"];

fn domain_matches(domain: &str, allowed_suffixes: &[&str]) -> bool {
    allowed_suffixes
        .iter()
        .any(|suffix| domain.ends_with(suffix))
}

/**
 * js反爬虫：
 * 1. 浏览器第一次访问，基于当前周(now_week)生成当前server端的dynamic_secret
 * 2. 浏览器通过js脚本生成visitorId，后端基于visitorId做一次校验
 */
async fn anti_bot(
    Component(sc_service): &Component<SystemConfigService>,
    cookies: &CookieJar,
    user_agent: UserAgent,
    client_ip: IpAddr,
) -> Option<Response> {
    let seo_user_agents = sc_service.parsed_seo_user_agents().await;
    if seo_user_agents
        .iter()
        .any(|seo_ua| user_agent.as_str().contains(seo_ua))
    {
        if let Some(bot_name) = validate_seo_ip(client_ip).await.ok().flatten() {
            tracing::trace!("confirmed to be from a legitimate crawler: {bot_name}");
            return None;
        }
    }
    let block_user_agents = sc_service.parsed_block_user_agents().await;
    if block_user_agents.iter().any(|block_ua| {
        user_agent
            .as_str()
            .to_lowercase()
            .contains(&block_ua.to_lowercase())
    }) {
        tracing::warn!("blocked user agent: {}", user_agent.as_str());
        return Some(StatusCode::FORBIDDEN.into_response());
    }

    let ip_blacklist = sc_service.parsed_ip_blacklist().await;
    if ip_blacklist.iter().any(|net| net.contains(&client_ip)) {
        tracing::warn!("blocked ip address: {}", client_ip);
        return Some(StatusCode::FORBIDDEN.into_response());
    }

    let server_secret = "server-secret";
    let client_ip = client_ip.to_string();
    let now_week = Utc::now().timestamp() / 60 / 60 / 24 / 7; // 当前周时间戳

    let mut hasher = Sha256::new();
    hasher.update(format!("{now_week}{client_ip}{server_secret}").as_bytes());
    let dynamic_secret = hex::encode(hasher.finalize());

    if let (Some(token), Some(fp)) = (cookies.get("x-anti-token"), cookies.get("x-fp")) {
        // 用 visitorId + dynamic_secret 生成期望 token
        let visitor_id = fp.value();
        let mut token_hasher = Sha256::new();
        token_hasher.update(format!("{now_week}{visitor_id}{dynamic_secret}").as_bytes());
        let expected_token = hex::encode(token_hasher.finalize());

        if expected_token == token.value() {
            // token 有效，允许访问
            return None;
        }
    }

    let template = AntiBotTemplate {
        server_secret_key: dynamic_secret.as_str(),
    };

    let html = template.render().ok()?;
    return Some((StatusCode::ACCEPTED, Html(html)).into_response());
}

async fn validate_seo_ip(client_ip: IpAddr) -> anyhow::Result<Option<&'static str>> {
    let resolver = hickory_resolver::Resolver::builder_with_config(
        hickory_resolver::config::ResolverConfig::default(),
        hickory_resolver::name_server::TokioConnectionProvider::default(),
    )
    .build();

    // Step 1: 反向解析 IP -> 域名
    let ptr_response = resolver.reverse_lookup(client_ip).await?;
    let hostname = match ptr_response.iter().next() {
        Some(name) => name.to_utf8(),
        None => return Ok(None),
    };

    // Step 2: 检查域名后缀
    let crawler_type = if domain_matches(&hostname, &GOOGLE_DOMAINS) {
        Some("Googlebot")
    } else if domain_matches(&hostname, &BING_DOMAINS) {
        Some("Bingbot")
    } else if domain_matches(&hostname, &BAIDU_DOMAINS) {
        Some("Baiduspider")
    } else if domain_matches(&hostname, &SOGOU_DOMAINS) {
        Some("SogouSpider")
    } else {
        None
    };

    if let Some(bot) = crawler_type {
        // Step 3: 正查 域名 -> IP
        let forward_response = resolver.lookup_ip(&hostname).await?;
        if forward_response
            .iter()
            .any(|resolved_ip| resolved_ip == client_ip)
        {
            return Ok(Some(bot));
        }
    }
    Ok(None)
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
    let prefix = if let Some(pos) = original_host.find(".dtiku.cn") {
        let prefix = &original_host[..pos]; // "gwy"
        if prefix == "www" {
            "gwy"
        } else {
            prefix
        }
    } else {
        "gwy"
    };
    let root_exam = ec_service
        .find_root_exam(prefix)
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
