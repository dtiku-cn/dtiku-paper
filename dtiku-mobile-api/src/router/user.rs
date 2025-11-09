use crate::router::{encode, Claims};
use crate::service::user::UserService;
use anyhow::Context;
use axum_extra::extract::cookie::{Cookie, SameSite};
use chrono::{Duration, Utc};
use cookie::time::Duration as CookieDuration;
use dtiku_base::model::user_info;
use schemars::JsonSchema;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DbConn};
use serde::{Deserialize, Serialize};
use spring_web::{
    axum::Json,
    error::{KnownWebError, Result},
    extractor::Component,
    get_api, post_api,
};
#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct UserResponse {
    pub id: i32,
    pub name: String,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub vip_level: i16,
    pub vip_expired_at: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct LoginResponse {
    pub user: UserResponse,
    pub token: String,
}

impl From<user_info::Model> for UserResponse {
    fn from(u: user_info::Model) -> Self {
        Self {
            id: u.id,
            name: u.name,
            email: Some(format!("{}@dtiku.cn", u.wechat_id)),
            avatar_url: Some(u.avatar),
            vip_level: 0,
            vip_expired_at: Some(u.expired),
        }
    }
}

/// POST /api/user/login
#[post_api("/api/user/login")]
async fn api_user_login(
    Component(us): Component<UserService>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>> {
    let user = us
        .find_user_by_name(&req.username)
        .await?
        .ok_or_else(|| KnownWebError::unauthorized("用户名或密码错误"))?;

    // TODO: 验证密码

    let claims = Claims {
        user_id: user.id,
        exp: (Utc::now() + Duration::days(30)).timestamp() as u64,
        iat: Utc::now().timestamp() as u64,
    };
    let token = encode(claims).context("token creation failed")?;

    Ok(Json(LoginResponse {
        user: user.into(),
        token,
    }))
}

/// POST /api/user/register
#[post_api("/api/user/register")]
async fn api_user_register(
    Component(db): Component<DbConn>,
    cookies: CookieJar,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<LoginResponse>> {
    let new_user = user_info::ActiveModel {
        name: Set(req.username.clone()),
        wechat_id: Set(req.username.clone()),
        avatar: Set("".to_string()),
        ..Default::default()
    };

    let user = new_user.insert(&db).await.context("创建用户失败")?;

    let claims = Claims {
        user_id: user.id,
        exp: (Utc::now() + Duration::days(30)).timestamp() as u64,
        iat: Utc::now().timestamp() as u64,
    };
    let token = encode(claims).context("token creation failed")?;

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
#[get_api("/api/user/info")]
async fn api_user_info(
    claims: Claims,
    Component(us): Component<UserService>,
) -> Result<Json<UserResponse>> {
    let user = us.get_user_detail(claims.user_id).await?;
    Ok(Json(UserResponse::from(user)))
}

/// POST /api/user/logout
#[post_api("/api/user/logout")]
async fn api_user_logout(cookies: CookieJar) -> Result<Json<serde_json::Value>> {
    let mut token_cookie = Cookie::new("token", "");
    token_cookie.set_domain(".dtiku.cn");
    token_cookie.set_path("/");
    token_cookie.set_max_age(CookieDuration::seconds(0));
    let cookies = cookies.add(token_cookie);

    Ok((cookies, Json(serde_json::json!({"success": true}))))
}
