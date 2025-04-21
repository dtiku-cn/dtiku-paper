use crate::data::question::QuestionSearchTemplate;
use anyhow::Context;
use askama::Template;
use spring_web::{axum::response::IntoResponse, error::Result, get};

#[get("/question/search")]
async fn search_question() -> Result<impl IntoResponse> {
    let t = QuestionSearchTemplate {};
    Ok(t.render().context("render failed")?)
}
