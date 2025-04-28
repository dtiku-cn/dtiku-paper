use crate::views::question::QuestionSearchTemplate;
use anyhow::Context;
use askama::Template;
use spring_web::{axum::response::{Html, IntoResponse}, error::Result, get};

#[get("/question/search")]
async fn search_question() -> Result<impl IntoResponse> {
    let t = QuestionSearchTemplate {};
    Ok(Html(t.render().context("render failed")?))
}
