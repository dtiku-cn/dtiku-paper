use crate::views::{
    paper::{ListPaperTemplate, PaperTemplate},
    GlobalVariables, IntoTemplate,
};
use anyhow::Context;
use askama::Template;
use dtiku_paper::{
    query::paper::ListPaperQuery,
    service::{exam_category::ExamCategoryService, label::LabelService, paper::PaperService},
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
    Query(mut query): Query<ListPaperQuery>,
    Component(ecs): Component<ExamCategoryService>,
    Component(ps): Component<PaperService>,
    Component(ls): Component<LabelService>,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let paper_type = query.paper_type;
    let current_paper_type = ecs
        .find_by_id_with_cache(paper_type)
        .await?
        .ok_or_else(|| {
            KnownWebError::bad_request(format!("试卷类型不存在:{}", query.paper_type))
        })?;
    let root_exam_category = ecs
        .find_root_exam_by_id(current_paper_type.pid)
        .await?
        .ok_or_else(|| {
            KnownWebError::bad_request(format!("试卷类型不存在:{}", query.paper_type))
        })?;

    let label_tree = ls.find_all_label_by_paper_type(paper_type).await?;

    if query.label_id == 0 {
        // 默认值
        query.label_id = label_tree.default_label_id();
    }
    let list = ps.find_paper_by_query(query).await?;
    let t = ListPaperTemplate::new(
        global,
        label_tree,
        current_paper_type,
        root_exam_category,
        list,
    );
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
