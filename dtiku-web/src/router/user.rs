use crate::{
    plugins::AuthConfig,
    router::{decode, error_messages, jwt},
    service::user::UserService,
    views::user::{ArtalkUser, UserLoginRefreshTemplate},
};
use anyhow::Context;
use askama::Template;
use axum_extra::extract::{
    cookie::{Cookie, SameSite},
    CookieJar,
};
use chrono::Utc;
use cookie::time::Duration;
use serde::{Deserialize, Serialize};
use sha1::Digest;
use spring_redis::redis::AsyncCommands;
use spring_redis::Redis;
use spring_web::{
    axum::{
        body::Bytes,
        http::HeaderMap,
        response::{Html, IntoResponse, Redirect, Json},
    },
    error::{KnownWebError, Result},
    extractor::{Component, Path, RawQuery},
    get, post,
};
use uuid::Uuid;

#[get("/api/v2/auth/{provider}/callback")]
async fn user_login_callback(
    Path(provider): Path<String>,
    RawQuery(query): RawQuery,
    Component(us): Component<UserService>,
    headers: HeaderMap,
    cookies: CookieJar,
) -> Result<impl IntoResponse> {
    let token = us
        .auth_callback(headers, provider, query.unwrap_or_default())
        .await
        .context("auth_callback error")?;
    let claims = decode(&token)?;
    let user = us.get_user_detail(claims.user_id).await?;
    let t = UserLoginRefreshTemplate {
        user: ArtalkUser {
            email: format!("{}@wechat.com", user.wechat_id),
            name: user.name,
            token: token.clone(),
            link: "".to_string(),
            is_admin: false,
        },
    };
    let mut token_cookie = Cookie::new("token", token);
    token_cookie.set_domain(".dtiku.cn"); // 站点下所有子域都能访问
    token_cookie.set_path("/"); // 所有请求路径都生效
    token_cookie.set_same_site(SameSite::Lax); // 部分跨站请求（如 GET 的链接跳转）可以携带 Cookie，适度平衡安全与体验。
    token_cookie.set_secure(true); // 仅 HTTPS 发送
    token_cookie.set_max_age(Duration::days(30));
    let cookies = cookies.add(token_cookie);
    Ok((cookies, Html(t.render().context("render failed")?)))
}

#[get("/user/comment/{comment_id}/avatar")]
async fn user_comment_avatar(
    Path(comment_id): Path<i64>,
    Component(us): Component<UserService>,
) -> Result<impl IntoResponse> {
    let avatar_url = us
        .get_comment_user_avatar(comment_id)
        .await?
        .ok_or_else(|| KnownWebError::not_found(error_messages::USER_AVATAR_NOT_FOUND))?;
    Ok(Redirect::permanent(&avatar_url))
}

// ==================== 微信公众号关注登录 ====================
// 相关文档:
// https://developers.weixin.qq.com/doc/service/api/qrcode/qrcodes/api_createqrcode.html
// https://developers.weixin.qq.com/doc/service/guide/product/message/Receiving_event_pushes.html
// https://developers.weixin.qq.com/doc/service/api/webdev/access/api_snsaccesstoken.html

#[derive(Debug, Serialize)]
struct QrcodeResponse {
    /// 二维码场景值ID
    scene_id: String,
    /// 二维码图片URL
    qrcode_url: String,
    /// 过期时间（秒）
    expire_seconds: i32,
}

/// 生成微信登录二维码
/// GET /api/wechat/login/qrcode
#[get("/api/wechat/login/qrcode")]
async fn wechat_login_qrcode(
    Component(us): Component<UserService>,
    Component(redis): Component<Redis>,
) -> Result<impl IntoResponse> {
    // 生成唯一的场景值 ID
    let scene_id = Uuid::new_v4().to_string();
    
    // 创建二维码
    let ticket = us
        .create_wechat_login_qrcode(&scene_id)
        .await
        .context("创建二维码失败")?;
    
    // 将场景值存储到 Redis，标记为待扫码状态
    let key = format!("wechat:login:scene:{}", scene_id);
    let _: () = redis
        .clone()
        .set_ex(&key, "pending", 600) // 10分钟有效期
        .await
        .context("Redis 操作失败")?;
    
    // 返回二维码信息
    Ok(Json(QrcodeResponse {
        scene_id,
        qrcode_url: format!("https://mp.weixin.qq.com/cgi-bin/showqrcode?ticket={}", ticket),
        expire_seconds: 600,
    }))
}

#[derive(Debug, Serialize)]
struct LoginStatusResponse {
    /// 登录状态: pending(待扫码), scanned(已扫码), success(登录成功), expired(已过期)
    status: String,
    /// 登录成功后的 token
    token: Option<String>,
    /// 用户信息
    user: Option<serde_json::Value>,
}

/// 查询登录状态
/// GET /api/wechat/login/status/:scene_id
#[get("/api/wechat/login/status/{scene_id}")]
async fn wechat_login_status(
    Path(scene_id): Path<String>,
    Component(redis): Component<Redis>,
    Component(us): Component<UserService>,
) -> Result<impl IntoResponse> {
    let key = format!("wechat:login:scene:{}", scene_id);
    
    let status: Option<String> = redis.clone().get(&key).await.context("Redis 操作失败")?;
    
    match status.as_deref() {
        None => {
            // 场景值不存在或已过期
            Ok(Json(LoginStatusResponse {
                status: "expired".to_string(),
                token: None,
                user: None,
            }))
        }
        Some("pending") => {
            // 待扫码
            Ok(Json(LoginStatusResponse {
                status: "pending".to_string(),
                token: None,
                user: None,
            }))
        }
        Some(user_id_str) if user_id_str.parse::<i32>().is_ok() => {
            // 已登录成功，返回用户信息和 token
            let user_id = user_id_str.parse::<i32>().unwrap();
            let user = us.get_user_detail(user_id).await?;
            
            // 生成 JWT token
            let claims = jwt::Claims {
                user_id,
                exp: (Utc::now() + chrono::Duration::days(30)).timestamp() as u64,
                iat: Utc::now().timestamp() as u64,
            };
            let token = jwt::encode(claims).context("生成 token 失败")?;
            
            // 删除 Redis 中的场景值
            let _: () = redis.clone().del(&key).await.context("Redis 操作失败")?;
            
            Ok(Json(LoginStatusResponse {
                status: "success".to_string(),
                token: Some(token),
                user: Some(serde_json::json!({
                    "id": user.id,
                    "name": user.name,
                    "avatar": user.avatar,
                })),
            }))
        }
        _ => {
            // 未知状态
            Ok(Json(LoginStatusResponse {
                status: "pending".to_string(),
                token: None,
                user: None,
            }))
        }
    }
}

// ==================== 微信工具函数 ====================

/// 验证微信签名
/// 
/// 微信签名验证算法：
/// 1. 将 token、timestamp、nonce 三个参数进行字典序排序
/// 2. 将三个参数拼接成一个字符串
/// 3. 对拼接后的字符串进行 SHA1 加密
/// 4. 将加密后的字符串与 signature 进行比较
fn verify_wechat_signature(
    token: &str,
    params: &std::collections::HashMap<String, String>,
) -> bool {
    // 如果 token 为空，跳过验证（仅用于开发环境）
    if token.is_empty() {
        spring::tracing::warn!("微信事件推送 token 未配置，跳过签名验证");
        return true;
    }
    
    // 获取签名参数
    let Some(signature) = params.get("signature") else {
        spring::tracing::warn!("缺少 signature 参数");
        return false;
    };
    
    let timestamp = params.get("timestamp").map(String::as_str).unwrap_or("");
    let nonce = params.get("nonce").map(String::as_str).unwrap_or("");
    
    // 将 token、timestamp、nonce 进行字典序排序并拼接
    let mut arr = [token, timestamp, nonce];
    arr.sort_unstable();
    let concatenated = arr.concat();
    
    // 计算 SHA1 签名
    let mut hasher = sha1::Sha1::new();
    hasher.update(concatenated.as_bytes());
    let hash_hex = hex::encode(hasher.finalize());
    
    // 比较签名
    let is_valid = hash_hex.eq_ignore_ascii_case(signature);
    
    if !is_valid {
        spring::tracing::warn!(
            "签名验证失败: 计算={}, 收到={}, timestamp={}, nonce={}",
            hash_hex,
            signature,
            timestamp,
            nonce
        );
    }
    
    is_valid
}

/// 微信事件消息结构
#[derive(Debug, Deserialize)]
#[serde(rename = "xml", rename_all = "PascalCase")]
#[allow(dead_code)]
struct WechatEventMessage {
    to_user_name: String,
    from_user_name: String,
    create_time: i64,
    msg_type: String,
    event: Option<String>,
    event_key: Option<String>,
    ticket: Option<String>,
}

/// 解析并验证微信请求参数
fn parse_and_verify_wechat_params(
    query: Option<String>,
    token: &str,
) -> Result<std::collections::HashMap<String, String>> {
    let params: std::collections::HashMap<String, String> = 
        serde_urlencoded::from_str(&query.unwrap_or_default())
            .context("解析查询参数失败")?;
    
    // 验证微信签名
    if !verify_wechat_signature(token, &params) {
        spring::tracing::warn!("微信签名验证失败: {:?}", params);
        return Err(KnownWebError::unauthorized("签名验证失败").into());
    }
    
    Ok(params)
}

/// 微信接入验证（GET 请求）
/// GET /api/wechat/callback
#[get("/api/wechat/callback")]
async fn wechat_verify_callback(
    RawQuery(query): RawQuery,
    Component(auth_config): Component<AuthConfig>,
) -> Result<impl IntoResponse> {
    spring::tracing::info!("收到微信接入验证请求");
    
    let params = parse_and_verify_wechat_params(query, &auth_config.wechat_mp_event_token)?;
    
    // 返回 echostr 完成验证
    let echostr = params
        .get("echostr")
        .ok_or_else(|| anyhow::anyhow!("缺少 echostr 参数"))?;
    
    spring::tracing::info!("微信接入验证成功");
    Ok(echostr.clone())
}

/// 接收微信事件推送（POST 请求）
/// POST /api/wechat/callback
#[post("/api/wechat/callback")]
async fn wechat_event_callback(
    RawQuery(query): RawQuery,
    Component(redis): Component<Redis>,
    Component(us): Component<UserService>,
    Component(auth_config): Component<AuthConfig>,
    body: Bytes,
) -> Result<impl IntoResponse> {
    // 验证签名
    let _params = parse_and_verify_wechat_params(query, &auth_config.wechat_mp_event_token)?;
    
    // 将 bytes 转换为 string
    let body_str = String::from_utf8(body.to_vec()).context("解析 body 失败")?;
    
    spring::tracing::info!("收到微信事件推送: {}", body_str);
    
    // 使用 quick-xml 解析 XML 消息
    let msg: WechatEventMessage = quick_xml::de::from_str(&body_str)
        .context("解析 XML 消息失败")?;
    
    spring::tracing::debug!("解析后的消息: {:?}", msg);
    
    let openid = &msg.from_user_name;
    
    // 处理扫码事件
    match msg.event.as_deref() {
        Some("subscribe") => {
            // 用户未关注时扫码：先关注，再推送此事件
            // EventKey 格式：qrscene_场景值
            if let Some(event_key) = &msg.event_key {
                let scene_id = event_key.strip_prefix("qrscene_").unwrap_or(event_key);
                
                spring::tracing::info!("用户 {} 通过场景 {} 关注了公众号", openid, scene_id);
                
                // 处理登录逻辑
                handle_wechat_login(&us, &redis, openid, scene_id).await?;
            }
        }
        Some("SCAN") => {
            // 用户已关注时扫码：直接推送此事件
            // EventKey 格式：场景值（不带 qrscene_ 前缀）
            if let Some(scene_id) = &msg.event_key {
                spring::tracing::info!("已关注用户 {} 扫描了场景 {}", openid, scene_id);
                
                // 处理登录逻辑
                handle_wechat_login(&us, &redis, openid, scene_id).await?;
            }
        }
        _ => {
            spring::tracing::debug!("忽略其他事件类型: {:?}", msg.event);
        }
    }
    
    // 返回 success 表示消息已处理
    Ok("success".to_string())
}

/// 处理微信登录逻辑（提取为公共函数）
async fn handle_wechat_login(
    us: &UserService,
    redis: &Redis,
    openid: &str,
    scene_id: &str,
) -> anyhow::Result<()> {
    use spring_redis::redis::AsyncCommands;
    
    const LOGIN_SCENE_TTL: u64 = 600; // 10 分钟
    
    // 获取用户信息
    let wechat_user = us
        .get_wechat_user_info(openid)
        .await
        .context("获取微信用户信息失败")?;
    
    // 创建或更新用户
    let user = us
        .find_or_create_wechat_user(&wechat_user)
        .await
        .context("创建或更新用户失败")?;
    
    // 更新 Redis 中的场景值状态，存储用户 ID
    let key = format!("wechat:login:scene:{}", scene_id);
    redis
        .clone()
        .set_ex(&key, user.id.to_string(), LOGIN_SCENE_TTL)
        .await
        .context("Redis 操作失败")?;
    
    spring::tracing::info!(
        "用户登录成功: name={}, id={}, openid={}, scene={}",
        user.name,
        user.id,
        openid,
        scene_id
    );
    
    Ok(())
}
