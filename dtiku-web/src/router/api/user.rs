use crate::{router::{encode, Claims}, service::user::UserService};
use anyhow::Context;
use axum_extra::extract::{
    cookie::{Cookie, SameSite},
    CookieJar,
};
use chrono::{Duration, Utc};
use cookie::time::Duration as CookieDuration;
use dtiku_base::model::user_info;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DbConn};
use serde::{Deserialize, Serialize};
use spring_web::{
    axum::{response::IntoResponse, Json},
    error::{KnownWebError, Result},
    extractor::Component,
    get, post,
};

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: i32,
    pub name: String,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub vip_level: i16,
    pub vip_expired_at: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user: UserResponse,
    pub token: String,
}

impl From<user_info::Model> for UserResponse {
    fn from(u: user_info::Model) -> Self {
        Self {
            id: u.id,
            name: u.name,
            email: Some(format!("{}@dtiku.cn", u.wechat_id)), // 从 wechat_id 生成临时邮箱
            avatar_url: Some(u.avatar),
            vip_level: 0, // user_info 模型没有 vip_level 字段，使用默认值
            vip_expired_at: Some(u.expired),
        }
    }
}

/// POST /api/user/login
#[post("/api/user/login")]
async fn api_user_login(
    Component(us): Component<UserService>,
    cookies: CookieJar,
    Json(req): Json<LoginRequest>,
) -> Result<impl IntoResponse> {
    // 简单实现：通过用户名查找用户（实际项目应该验证密码）
    let user = us
        .find_user_by_name(&req.username)
        .await?
        .ok_or_else(|| KnownWebError::unauthorized("用户名或密码错误"))?;

    // TODO: 实际项目中应该验证密码
    // 这里暂时简化处理

    // 生成 JWT token
    let claims = Claims {
        user_id: user.id,
        exp: (Utc::now() + Duration::days(30)).timestamp() as u64,
        iat: Utc::now().timestamp() as u64,
    };
    let token = encode(claims).context("token creation failed")?;

    // 设置 cookie
    let mut token_cookie = Cookie::new("token", token.clone());
    token_cookie.set_domain(".dtiku.cn");
    token_cookie.set_path("/");
    token_cookie.set_same_site(SameSite::Lax);
    token_cookie.set_secure(true);
    token_cookie.set_max_age(CookieDuration::days(30));
    let cookies = cookies.add(token_cookie);

    Ok((
        cookies,
        Json(LoginResponse {
            user: user.into(),
            token,
        }),
    ))
}

/// POST /api/user/register
#[post("/api/user/register")]
async fn api_user_register(
    Component(db): Component<DbConn>,
    cookies: CookieJar,
    Json(req): Json<RegisterRequest>,
) -> Result<impl IntoResponse> {
    // 创建新用户
    let new_user = user_info::ActiveModel {
        name: Set(req.username.clone()),
        wechat_id: Set(req.username.clone()), // 临时使用用户名作为 wechat_id
        avatar: Set("".to_string()),
        ..Default::default()
    };

    let user = new_user
        .insert(&db)
        .await
        .context("创建用户失败")?;

    // 生成 JWT token
    let claims = Claims {
        user_id: user.id,
        exp: (Utc::now() + Duration::days(30)).timestamp() as u64,
        iat: Utc::now().timestamp() as u64,
    };
    let token = encode(claims).context("token creation failed")?;

    // 设置 cookie
    let mut token_cookie = Cookie::new("token", token.clone());
    token_cookie.set_domain(".dtiku.cn");
    token_cookie.set_path("/");
    token_cookie.set_same_site(SameSite::Lax);
    token_cookie.set_secure(true);
    token_cookie.set_max_age(CookieDuration::days(30));
    let cookies = cookies.add(token_cookie);

    Ok((
        cookies,
        Json(LoginResponse {
            user: user.into(),
            token,
        }),
    ))
}

/// GET /api/user/info
#[get("/api/user/info")]
async fn api_user_info(
    claims: Claims,
    Component(us): Component<UserService>,
) -> Result<impl IntoResponse> {
    let user = us.get_user_detail(claims.user_id).await?;
    Ok(Json(UserResponse::from(user)))
}

/// POST /api/user/logout
#[post("/api/user/logout")]
async fn api_user_logout(cookies: CookieJar) -> Result<impl IntoResponse> {
    // 清除 token cookie
    let mut token_cookie = Cookie::new("token", "");
    token_cookie.set_domain(".dtiku.cn");
    token_cookie.set_path("/");
    token_cookie.set_max_age(CookieDuration::seconds(0));
    let cookies = cookies.add(token_cookie);

    Ok((cookies, Json(serde_json::json!({"success": true}))))
}

