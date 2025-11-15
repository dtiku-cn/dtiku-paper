use crate::plugins::DtikuConfig;
use crate::router::jwt::OptionalClaims;
use crate::service::traffic::TrafficService;
use crate::service::user::UserService;
use crate::views::{AntiBotTemplate, ErrorTemplate, GlobalVariables};
use askama::Template;
use axum_client_ip::ClientIp;
use axum_extra::extract::{CookieJar, Host};
use axum_extra::headers::UserAgent;
use axum_extra::TypedHeader;
use chrono::Utc;
use dtiku_base::service::system_config::SystemConfigService;
use dtiku_paper::service::exam_category::ExamCategoryService;
use http::HeaderValue;
use sha2::{Digest, Sha256};
use spring::tracing;
use spring_web::axum::body;
use spring_web::axum::http::StatusCode;
use spring_web::axum::middleware::Next;
use spring_web::axum::response::{Html, IntoResponse, Response};
use spring_web::error::{KnownWebError, WebError};
use spring_web::extractor::{Component, Config, Request};
use std::net::IpAddr;
use tokio::task_local;

pub static IGNORE_PREFIX: [&str; 3] = ["/api", "/pay", "/assets"];

pub async fn global_error_page(
    mut tf_service: Component<TrafficService>,
    ec_service: Component<ExamCategoryService>,
    sc_service: Component<SystemConfigService>,
    us_service: Component<UserService>,
    dtiku_config: Config<DtikuConfig>,
    ClientIp(client_ip): ClientIp,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    claims: OptionalClaims,
    host: Host,
    cookies: CookieJar,
    req: Request,
    next: Next,
) -> Response {
    let original_host = host.0.clone();
    if !IGNORE_PREFIX
        .iter()
        .any(|prefix| req.uri().path().starts_with(prefix))
    {
        if let Some(resp) = anti_bot(
            &mut tf_service,
            &sc_service,
            &cookies,
            user_agent,
            client_ip,
            &original_host,
        )
        .await
        {
            return resp;
        }
    }
    let fp = cookies
        .get("x-fp")
        .map(|x_fp_id| x_fp_id.value())
        .map(|x_fp_id| format!("fp:{x_fp_id}"))
        .unwrap_or("".to_string());
    let resp = match with_context(
        &ec_service,
        &sc_service,
        &us_service,
        &dtiku_config,
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

/// 爬虫域名配置：(爬虫名称, 域名列表)
static CRAWLER_CONFIGS: &[(&str, &[&str])] = &[
    ("Googlebot", &["googlebot.com", "google.com"]),
    ("Bingbot", &["search.msn.com"]),
    ("Baiduspider", &["baidu.com"]),
    ("SogouSpider", &["sogou.com"]),
];

fn domain_matches(domain: &str, allowed_suffixes: &[&str]) -> bool {
    allowed_suffixes
        .iter()
        .any(|suffix| domain.ends_with(suffix))
}

/**
 * js反爬虫：
 * 1. 浏览器第一次访问，基于当前周(now_week)生成当前server端的dynamic_secret
 * 2. 浏览器通过js脚本生成visitorId，后端基于visitorId做一次校验
 *
 * arroyo流量实时监测，超过阈值加入redis block_ip列表
 * cap验证成功，从列表中移除ip
 * cap验证失败，加入数据库ip黑名单
 */
async fn anti_bot(
    Component(traffic_service): &mut Component<TrafficService>,
    Component(sc_service): &Component<SystemConfigService>,
    cookies: &CookieJar,
    user_agent: UserAgent,
    client_ip: IpAddr,
    original_host: &str,
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

    if traffic_service.is_block_ip(original_host, client_ip).await {
        if let Some(cap_token) = cookies.get("cap-token") {
            if traffic_service
                .verify_token(cap_token.value())
                .await
                .ok()
                .unwrap_or(false)
            {
                // cap验证成功，从列表中移除ip
                traffic_service.unblock_ip(original_host, client_ip).await;
                return None;
            }
        }

        tracing::warn!("realtime blocked ip address: {}", client_ip);
        let html = traffic_service.gen_cap_template().ok()?;
        return Some((StatusCode::ACCEPTED, Html(html)).into_response());
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
    let crawler_type = CRAWLER_CONFIGS
        .iter()
        .find(|(_, domains)| domain_matches(&hostname, domains))
        .map(|(name, _)| *name);

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

pub async fn not_found_handler(Host(original_host): Host) -> Response {
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
    Config(config): &Config<DtikuConfig>,
    OptionalClaims(claims): &OptionalClaims,
    Host(original_host): Host,
    cookies: CookieJar,
    mut req: Request,
    next: Next,
) -> Result<Response, WebError> {
    let prefix = if let Some(pos) = original_host.find(".dtiku.cn") {
        let prefix = &original_host[..pos]; // "gwy"
        let prefix = if let Some(strip_prefix) = prefix.strip_prefix(&config.strip_prefix) {
            strip_prefix
        } else {
            prefix
        };
        let prefix = prefix.trim_start_matches('.');
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
