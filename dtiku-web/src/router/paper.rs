use crate::views::{
    paper::{ListPaperTemplate, PaperTemplate},
    GlobalVariables, IntoTemplate,
};
use anyhow::Context;
use askama::Template;
use dtiku_paper::{
    query::paper::ListPaperQuery,
    service::{exam_category::ExamCategoryService, paper::PaperService},
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

#[get("/paper")]
async fn list_paper(
    Query(query): Query<ListPaperQuery>,
    Component(ecs): Component<ExamCategoryService>,
    Component(ps): Component<PaperService>,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let p = ecs
        .find_by_id_with_cache(query.paper_type)
        .await?
        .ok_or_else(|| KnownWebError::bad_request(format!("试卷类型不存在:{}", query.paper_type)))?;

    if query.label_id == 0 { // 默认值
        
    }
    println!("index");
    let list = ps.find_paper_by_query(query).await?;
    let t: ListPaperTemplate = list.to_template(global);
    Ok(Html(t.render().context("render failed")?))
}

#[get("/paper/{id}")]
async fn paper_by_id(
    Path(id): Path<i32>,
    Component(ps): Component<PaperService>,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    println!("paper: {id}");
    let paper = ps
        .find_paper_by_id(id)
        .await?
        .ok_or_else(|| KnownWebError::not_found("试卷未找到"))?;
    let t: PaperTemplate = paper.to_template(global);
    Ok(Html(t.render().context("render failed")?))
}
