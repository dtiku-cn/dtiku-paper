use crate::data::bbs::ListIssueTemplate;
use anyhow::Context;
use askama::Template;
use spring_web::{axum::response::IntoResponse, get, error::Result};

#[get("/bbs")]
async fn list_issue() -> Result<impl IntoResponse> {
    let t = ListIssueTemplate {};
    Ok(t.render().context("render failed")?)
}
