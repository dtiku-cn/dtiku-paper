use crate::{
    router::decode,
    service::user::UserService,
    views::user::{CurrentUser, UserLoginRefreshTemplate},
};
use anyhow::Context;
use askama::Template;
use spring_web::{
    axum::response::{Html, IntoResponse},
    error::Result,
    extractor::{Component, Path, RawQuery},
    get,
};

#[get("/api/v2/auth/{provider}/callback")]
async fn user_login_callback(
    Path(provider): Path<String>,
    RawQuery(query): RawQuery,
    Component(us): Component<UserService>,
) -> Result<impl IntoResponse> {
    let token = us
        .auth_callback(provider, query.unwrap_or_default())
        .await
        .context("auth_callback error")?;
    let claims = decode(&token)?;
    let user = us.get_user_detail(claims.user_id).await?;
    let t = UserLoginRefreshTemplate {
        user: CurrentUser {
            name: user.name,
            avatar: user.avatar,
        },
    };
    Ok(Html(t.render().context("render failed")?))
}
