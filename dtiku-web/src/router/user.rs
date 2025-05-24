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
        response::{Html, IntoResponse},
    },
    error::Result,
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
    let cookies = cookies.add(
        Cookie::build(("token", token))
            .same_site(SameSite::Lax)
            .http_only(true)
            .secure(true)
            .max_age(Duration::days(30))
            .build(),
    );
    Ok((cookies, Html(t.render().context("render failed")?)))
}
