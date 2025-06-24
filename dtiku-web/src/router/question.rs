use crate::{
    query::question::DetailQuery,
    router::EXAM_ID,
    views::{
        question::{
            OnlyCommentTemplate, QuestionDetailTemplate, QuestionSearchImgTemplate,
            QuestionSearchTemplate, QuestionSectionTemplate,
        },
        GlobalVariables,
    },
};
use anyhow::Context;
use askama::Template;
use dtiku_paper::{
    domain::question::QuestionSearch,
    query::question::PaperQuestionQuery,
    service::{label::LabelService, question::QuestionService},
};
use spring_web::{
    axum::{
        response::{Html, IntoResponse},
        Extension,
    },
    error::{KnownWebError, Result},
    extractor::{Component, Path, Query},
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
    let t = QuestionSearchImgTemplate {
        global,
        questions,
        query,
    };
    Ok(Html(t.render().context("render failed")?))
}

#[get("/question/section")]
async fn question_section(
    mut query: axum_extra::extract::Query<PaperQuestionQuery>,
    Component(qs): Component<QuestionService>,
    Component(ls): Component<LabelService>,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    if query.paper_type == 0 {
        let paper_type = global
            .get_paper_type_by_prefix("xingce")
            .ok_or_else(|| KnownWebError::bad_request("请指定试卷类型"))?;
        query.paper_type = paper_type.id;
    }
    let label_tree = ls.find_all_label_by_paper_type(query.paper_type).await?;
    let (questions, papers) = qs.search_question_by_section(&query).await?;
    let t = QuestionSectionTemplate {
        global,
        papers,
        questions,
        label_tree,
    };
    Ok(Html(t.render().context("render failed")?))
}

#[get("/question/recommend/{id}")]
async fn question_recommend(Path(id): Path<i32>) -> Result<impl IntoResponse> {
    Ok(format!("recommend/{id}: 等待上线中..."))
}

#[get("/question/detail/{id}")]
async fn question_detail(
    Path(id): Path<i32>,
    Query(q): Query<DetailQuery>,
    Component(qs): Component<QuestionService>,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    if q.only_comment {
        let t = OnlyCommentTemplate { global };
        Ok(Html(t.render().context("render failed")?))
    } else {
        let question = qs
            .full_question_by_id(id)
            .await?
            .ok_or_else(|| KnownWebError::not_found("题目不存在"))?;
        let t = QuestionDetailTemplate { global, question };
        Ok(Html(t.render().context("render failed")?))
    }
}
