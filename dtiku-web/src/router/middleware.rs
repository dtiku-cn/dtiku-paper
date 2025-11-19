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
use std::sync::OnceLock;
use tokio::task_local;

/// 不需要反爬虫检查的路径前缀
pub static IGNORE_PREFIX: [&str; 3] = ["/api", "/pay", "/assets"];

/// JS 反爬虫的服务端密钥
const SERVER_SECRET: &str = "server-secret";

/// 时间窗口：周（秒）
const TIME_WINDOW_SECONDS: i64 = 60 * 60 * 24 * 7;

/// HTML 片段请求标识
const FRAGMENT_ACCEPT_HEADER: &str = "text/html+fragment";

/// 片段请求时的页面刷新脚本
const FRAGMENT_RELOAD_SCRIPT: &str = r#"<script>window.location.reload();</script>"#;

pub async fn global_error_page(
    ec_service: Component<ExamCategoryService>,
    sc_service: Component<SystemConfigService>,
    us_service: Component<UserService>,
    dtiku_config: Config<DtikuConfig>,
    claims: OptionalClaims,
    host: Host,
    cookies: CookieJar,
    req: Request,
    next: Next,
) -> Response {
    let original_host = host.0.clone();
    let fp = cookies
        .get("x-fp")
        .map(|x_fp_id| x_fp_id.value())
        .map(|x_fp_id| format!("fp:{x_fp_id}"))
        .unwrap_or("".to_string());

    let mut resp = next.run(req).await;
    let remote_user = match &*claims {
        Some(c) => format!("u:{}", c.user_id),
        None => fp,
    };
    if let Ok(remote_user) = HeaderValue::from_str(&remote_user) {
        resp.headers_mut().insert("X-Remote-User", remote_user);
    }

    // 处理错误响应，渲染错误页面
    let status = resp.status();
    if status.is_client_error() || status.is_server_error() {
        let msg = resp.into_body();
        match body::to_bytes(msg, usize::MAX).await {
            Ok(bytes) => {
                let msg = String::from_utf8(bytes.to_vec())
                    .unwrap_or_else(|_| "Invalid UTF-8 in response body".to_string());
                let t = ErrorTemplate {
                    status,
                    msg: msg.as_str(),
                    original_host: original_host.as_str(),
                };
                match t.render() {
                    Ok(html) => Html(html).into_response(),
                    Err(e) => {
                        tracing::error!("render error template failed: {e:?}");
                        (status, msg).into_response()
                    }
                }
            }
            Err(e) => {
                tracing::error!("failed to read response body: {e:?}");
                (status, "Failed to read error message").into_response()
            }
        }
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

/// 全局 DNS Resolver 实例（延迟初始化）
static DNS_RESOLVER: OnceLock<
    hickory_resolver::Resolver<hickory_resolver::name_server::TokioConnectionProvider>,
> = OnceLock::new();

/// 获取或创建 DNS Resolver
fn get_dns_resolver(
) -> &'static hickory_resolver::Resolver<hickory_resolver::name_server::TokioConnectionProvider> {
    DNS_RESOLVER.get_or_init(|| {
        hickory_resolver::Resolver::builder_with_config(
            hickory_resolver::config::ResolverConfig::default(),
            hickory_resolver::name_server::TokioConnectionProvider::default(),
        )
        .build()
    })
}

/// 检查是否为 HTML 片段请求
fn is_fragment_request(req: &Request) -> bool {
    req.headers()
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains(FRAGMENT_ACCEPT_HEADER))
        .unwrap_or(false)
}

/// 生成片段请求或完整页面的响应
fn create_anti_bot_response(
    is_fragment: bool,
    full_html: impl FnOnce() -> Option<String>,
) -> Option<Response> {
    let html = if is_fragment {
        FRAGMENT_RELOAD_SCRIPT.to_string()
    } else {
        full_html()?
    };
    Some((StatusCode::ACCEPTED, Html(html)).into_response())
}

fn domain_matches(domain: &str, allowed_suffixes: &[&str]) -> bool {
    allowed_suffixes
        .iter()
        .any(|suffix| domain.ends_with(suffix))
}

/// 多层反爬虫机制
///
/// ## 策略层级：
/// 1. **SEO 爬虫白名单验证**：通过反向 DNS 验证合法爬虫（Google、Bing 等）
/// 2. **User Agent 黑名单**：阻止已知恶意 UA
/// 3. **IP 黑名单**：阻止配置的恶意 IP 段
/// 4. **JS 反爬虫**：验证浏览器 JS 执行能力（防无 JS 直接请求）
///    - 基于当前周(now_week)生成动态密钥
///    - 浏览器通过 JS 生成 visitorId 和 token
/// 5. **Captcha 反爬虫**：防无头浏览器（基于 Arroyo 实时流量监测）
///    - 超阈值的 IP 进入 Redis block_ip 列表
///    - 验证成功后从列表移除
pub async fn anti_bot(
    Component(mut traffic_service): Component<TrafficService>,
    Component(sc_service): Component<SystemConfigService>,
    ClientIp(client_ip): ClientIp,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    Host(original_host): Host,
    cookies: CookieJar,
    req: Request,
    next: Next,
) -> Response {
    let is_fragment = is_fragment_request(&req);

    // 对非忽略路径执行反爬虫检查
    if IGNORE_PREFIX
        .iter()
        .any(|prefix| req.uri().path().starts_with(prefix))
    {
        return next.run(req).await;
    }
    // 1. SEO 爬虫白名单验证
    if should_verify_as_seo_bot(&sc_service, &user_agent).await {
        if let Some(bot_name) = validate_seo_ip(client_ip).await.ok().flatten() {
            tracing::trace!("confirmed to be from a legitimate crawler: {bot_name}");
            return next.run(req).await;
        }
    }

    // 2. User Agent 黑名单检查
    if is_blocked_user_agent(&sc_service, &user_agent).await {
        tracing::warn!("blocked user agent: {}", user_agent.as_str());
        return StatusCode::FORBIDDEN.into_response();
    }

    // 3. IP 黑名单检查
    if is_blocked_ip(&sc_service, client_ip).await {
        tracing::warn!("blocked ip address: {}", client_ip);
        return StatusCode::FORBIDDEN.into_response();
    }

    // 4. JS 反爬虫验证
    if let Some(resp) = js_anti_bot(&cookies, client_ip, is_fragment) {
        return resp;
    }

    // 5. Captcha 反爬虫验证（实时流量监控）
    if let Some(resp) = captcha_anti_bot(
        &mut traffic_service,
        &cookies,
        client_ip,
        &original_host,
        is_fragment,
    )
    .await
    {
        return resp;
    }

    next.run(req).await
}

/// 检查是否需要验证为 SEO 爬虫
async fn should_verify_as_seo_bot(
    sc_service: &SystemConfigService,
    user_agent: &UserAgent,
) -> bool {
    let seo_user_agents = sc_service.parsed_seo_user_agents().await;
    seo_user_agents
        .iter()
        .any(|seo_ua| user_agent.as_str().contains(seo_ua))
}

/// 检查 User Agent 是否在黑名单中
async fn is_blocked_user_agent(sc_service: &SystemConfigService, user_agent: &UserAgent) -> bool {
    let block_user_agents = sc_service.parsed_block_user_agents().await;
    let ua_lower = user_agent.as_str().to_lowercase();
    block_user_agents
        .iter()
        .any(|block_ua| ua_lower.contains(&block_ua.to_lowercase()))
}

/// 检查 IP 是否在黑名单中
async fn is_blocked_ip(sc_service: &SystemConfigService, client_ip: IpAddr) -> bool {
    let ip_blacklist = sc_service.parsed_ip_blacklist().await;
    ip_blacklist.iter().any(|net| net.contains(&client_ip))
}

async fn captcha_anti_bot(
    traffic_service: &mut TrafficService,
    cookies: &CookieJar,
    client_ip: IpAddr,
    original_host: &str,
    is_fragment: bool,
) -> Option<Response> {
    // 检查 IP 是否在实时封禁列表中
    if !traffic_service.is_block_ip(original_host, client_ip).await {
        return None;
    }

    // 验证 captcha token
    if let Some(cap_token) = cookies.get("cap-token") {
        tracing::info!("verify cap-token: {}", cap_token.value());
        if traffic_service
            .verify_token(cap_token.value())
            .await
            .ok()
            .unwrap_or(false)
        {
            tracing::info!("verify cap-token successful: {}", cap_token.value());
            // cap验证成功，从列表中移除ip
            traffic_service.unblock_ip(original_host, client_ip).await;
            return None;
        }
    }

    tracing::warn!("realtime blocked ip address: {}", client_ip);

    // 返回验证页面或刷新脚本
    create_anti_bot_response(is_fragment, || traffic_service.gen_cap_template().ok())
}

fn js_anti_bot(cookies: &CookieJar, client_ip: IpAddr, is_fragment: bool) -> Option<Response> {
    let client_ip_str = client_ip.to_string();
    let now_week = Utc::now().timestamp() / TIME_WINDOW_SECONDS;

    // 生成动态密钥
    let dynamic_secret = generate_hash(&format!("{now_week}{client_ip_str}{SERVER_SECRET}"));

    // 验证 token
    if let (Some(token), Some(fp)) = (cookies.get("x-anti-token"), cookies.get("x-fp")) {
        let visitor_id = fp.value();
        let expected_token = generate_hash(&format!("{now_week}{visitor_id}{dynamic_secret}"));

        if expected_token == token.value() {
            // token 有效，允许访问
            return None;
        }
    }

    // 返回反爬虫验证页面或刷新脚本
    create_anti_bot_response(is_fragment, || {
        let template = AntiBotTemplate {
            server_secret_key: dynamic_secret.as_str(),
        };
        template.render().ok()
    })
}

/// 生成 SHA256 哈希值
fn generate_hash(data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    hex::encode(hasher.finalize())
}

/// 验证 SEO 爬虫 IP 的合法性
///
/// 通过反向 DNS 查询和正向 DNS 查询来验证爬虫的真实性：
/// 1. 反向解析 IP -> 域名
/// 2. 检查域名是否匹配已知爬虫的域名后缀
/// 3. 正向解析域名 -> IP，验证是否与原始 IP 一致
async fn validate_seo_ip(client_ip: IpAddr) -> anyhow::Result<Option<&'static str>> {
    let resolver = get_dns_resolver();

    // Step 1: 反向解析 IP -> 域名
    let ptr_response = resolver.reverse_lookup(client_ip).await?;
    let hostname = match ptr_response.iter().next() {
        Some(name) => name.to_utf8(),
        None => return Ok(None),
    };

    // Step 2: 检查域名后缀，匹配已知爬虫
    let crawler_type = CRAWLER_CONFIGS
        .iter()
        .find(|(_, domains)| domain_matches(&hostname, domains))
        .map(|(name, _)| *name);

    if let Some(bot) = crawler_type {
        // Step 3: 正向解析域名 -> IP，验证一致性
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
    match t.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("render not found template failed: {e:?}");
            (StatusCode::NOT_FOUND, "Page not found").into_response()
        }
    }
}

task_local! {
    pub static EXAM_ID: i16;
}

/// 为每个请求注入全局上下文变量
pub async fn with_global_context(
    ec_service: Component<ExamCategoryService>,
    sc_service: Component<SystemConfigService>,
    us_service: Component<UserService>,
    config: Config<DtikuConfig>,
    claims: OptionalClaims,
    host: Host,
    cookies: CookieJar,
    req: Request,
    next: Next,
) -> Response {
    match with_context_inner(
        &ec_service,
        &sc_service,
        &us_service,
        &config,
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
    }
}

/// 为每个请求注入全局上下文变量
#[inline]
async fn with_context_inner(
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
