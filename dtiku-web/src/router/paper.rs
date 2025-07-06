use crate::{
    query::paper::{ListPaperQuery, PaperQuery, PaperTitleLikeQuery},
    router::EXAM_ID,
    views::{
        paper::{ChapterPaperTemplate, ClusterPaperTemplate, ListPaperTemplate},
        GlobalVariables, IntoTemplate,
    },
};
use anyhow::Context;
use askama::Template;
use dtiku_paper::{
    domain::paper::PaperMode,
    model::paper::PaperExtra,
    query::paper::ListPaperQuery as PaperListQuery,
    service::{label::LabelService, paper::PaperService},
};
use spring_web::{
    axum::{
        response::{Html, IntoResponse},
        Extension, Json,
    },
    error::{KnownWebError, Result},
    extractor::{Component, Path, Query},
    get, post,
};

#[get("/paper")]
async fn list_paper(
    Query(query): Query<ListPaperQuery>,
    Component(ps): Component<PaperService>,
    Component(ls): Component<LabelService>,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let paper_type_prefix = query.paper_type_prefix;

    let paper_type = global
        .get_paper_type_by_prefix(&paper_type_prefix)
        .ok_or_else(|| KnownWebError::bad_request("试卷类型不存在"))?;

    let label_tree = ls.find_all_label_by_paper_type(paper_type.id).await?;

    let query = if query.label_id == 0 {
        PaperListQuery {
            paper_type: paper_type.id,
            label_id: label_tree.default_label_id(),
            page: query.page,
        }
    } else {
        PaperListQuery {
            paper_type: paper_type.id,
            label_id: query.label_id,
            page: query.page,
        }
    };
    let label = label_tree.get_label(query.label_id);
    let list = ps.find_paper_by_query(&query).await?;
    let t = ListPaperTemplate::new(global, query, label_tree, paper_type, label, list);
    Ok(Html(t.render().context("render failed")?))
}

#[get("/paper/{id}")]
async fn paper_by_id(
    Path(id): Path<i32>,
    Query(query): Query<PaperQuery>,
    Component(ps): Component<PaperService>,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let paper = ps
        .find_paper_by_id(id, query.mode.unwrap_or_default())
        .await?
        .ok_or_else(|| KnownWebError::not_found("试卷未找到"))?;
    let html = match paper.p.extra {
        PaperExtra::Chapters(_) => {
            let t: ChapterPaperTemplate = paper.to_template(global);
            t.render().context("render failed")?
        }
        _ => {
            let t: ClusterPaperTemplate = paper.to_template(global);
            t.render().context("render failed")?
        }
    };
    Ok(Html(html))
}

#[post("/paper/{id}/report")]
async fn paper_exercise(
    Path(id): Path<i32>,
    Component(ps): Component<PaperService>,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let paper = ps
        .find_paper_by_id(id, PaperMode::Exercise)
        .await?
        .ok_or_else(|| KnownWebError::not_found("试卷未找到"))?;
    let t: ChapterPaperTemplate = paper.to_template(global);
    Ok(Html(t.render().context("render failed")?))
}

#[get("/paper/title/like")]
async fn paper_title_like(
    Component(ps): Component<PaperService>,
    Query(query): Query<PaperTitleLikeQuery>,
) -> Result<impl IntoResponse> {
    let exam_id = EXAM_ID.get();
    let ps = ps.search_by_name(exam_id, &query.title).await?;
    Ok(Json(ps))
}
