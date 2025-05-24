use crate::{
    router::decode,
    service::user::UserService,
    views::user::{ArtalkUser, UserLoginRefreshTemplate},
};
use anyhow::Context;
use askama::Template;
use axum_extra::extract::{
    cookie::{Cookie, SameSite},
    CookieJar,
};
use cookie::time::Duration;
use spring_web::{
    axum::{
        http::HeaderMap,
        response::{Html, IntoResponse, Redirect},
    },
    error::{KnownWebError, Result},
    extractor::{Component, Path, RawQuery},
    get,
};

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
        .ok_or_else(|| KnownWebError::not_found("用户头像不存在"))?;
    Ok(Redirect::permanent(&avatar_url))
}
