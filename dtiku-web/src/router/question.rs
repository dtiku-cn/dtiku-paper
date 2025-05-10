use crate::views::{question::QuestionSearchTemplate, GlobalVariables};
use anyhow::Context;
use askama::Template;
use spring_web::{
    axum::{
        response::{Html, IntoResponse},
        Extension,
    },
    error::Result,
    get,
};

#[get("/question/search")]
async fn search_question(
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let t = QuestionSearchTemplate { global };
    Ok(Html(t.render().context("render failed")?))
}
