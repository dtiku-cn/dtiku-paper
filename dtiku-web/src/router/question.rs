use crate::views::{
    question::{QuestionSearchTemplate, QuestionSectionTemplate},
    GlobalVariables,
};
use anyhow::Context;
use askama::Template;
use dtiku_paper::query::question::PaperQuestionQuery;
use spring_web::{
    axum::{
        response::{Html, IntoResponse},
        Extension,
    },
    error::Result,
    extractor::Query,
    get,
};

#[get("/question/search")]
async fn search_question(
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let t = QuestionSearchTemplate { global };
    Ok(Html(t.render().context("render failed")?))
}

#[get("/question/section")]
async fn question_section(
    Query(query): Query<PaperQuestionQuery>,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let t = QuestionSectionTemplate { global };
    Ok(Html(t.render().context("render failed")?))
}
