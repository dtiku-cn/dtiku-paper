use crate::views::bbs::ListIssueTemplate;
use anyhow::Context;
use askama::Template;
use spring_web::{axum::response::{Html, IntoResponse}, error::Result, get};

#[get("/bbs")]
async fn list_issue() -> Result<impl IntoResponse> {
    let t = ListIssueTemplate {};
    Ok(Html(t.render().context("render failed")?))
}
