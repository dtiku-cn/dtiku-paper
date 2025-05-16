use crate::{
    router::EXAM_ID,
    views::{
        question::{QuestionSearchImgTemplate, QuestionSearchTemplate, QuestionSectionTemplate},
        GlobalVariables,
    },
};
use anyhow::Context;
use askama::Template;
use dtiku_paper::{
    domain::question::QuestionSearch, query::question::PaperQuestionQuery,
    service::question::QuestionService,
};
use spring_web::{
    axum::{
        response::{Html, IntoResponse},
        Extension,
    },
    error::Result,
    extractor::{Component, Query},
    get,
};

#[get("/question/search")]
async fn search_question(
    Extension(global): Extension<GlobalVariables>,
    Query(mut query): Query<QuestionSearch>,
    Component(qs): Component<QuestionService>,
) -> Result<impl IntoResponse> {
    let questions = if query.content.is_empty() {
        vec![]
    } else {
        query.exam_id = Some(EXAM_ID.get());
        qs.search_question(&query).await?
    };
    println!("{:?}", questions.clone());
    let t = QuestionSearchTemplate {
        global,
        questions,
        query,
    };
    Ok(Html(t.render().context("render failed")?))
}

#[get("/question/search/image")]
async fn search_question_by_img(
    Extension(global): Extension<GlobalVariables>,
    Query(mut query): Query<QuestionSearch>,
    Component(qs): Component<QuestionService>,
) -> Result<impl IntoResponse> {
    let questions = if query.content.is_empty() {
        vec![]
    } else {
        query.exam_id = Some(EXAM_ID.get());
        qs.search_question(&query).await?
    };
    println!("{:?}", questions.clone());
    let t = QuestionSearchImgTemplate {
        global,
        questions,
        query,
    };
    Ok(Html(t.render().context("render failed")?))
}

#[get("/question/section")]
async fn question_section(
    Query(_query): Query<PaperQuestionQuery>,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let t = QuestionSectionTemplate { global };
    Ok(Html(t.render().context("render failed")?))
}
