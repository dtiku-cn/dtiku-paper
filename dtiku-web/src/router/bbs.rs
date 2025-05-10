use crate::views::{bbs::ListIssueTemplate, GlobalVariables};
use anyhow::Context;
use askama::Template;
use spring_web::{
    axum::{response::{Html, IntoResponse}, Extension},
    error::Result,
    get,
};

#[get("/bbs")]
async fn list_issue(Extension(global): Extension<GlobalVariables>) -> Result<impl IntoResponse> {
    let t = ListIssueTemplate { global };
    Ok(Html(t.render().context("render failed")?))
}
