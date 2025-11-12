use std::collections::HashMap;

use crate::{
    query::paper::{ListPaperQuery, PaperQuery, PaperTitleLikeQuery},
    views::{
        paper::{ChapterPaperTemplate, ClusterPaperTemplate, ListPaperTemplate},
        GlobalVariables, IntoTemplate,
    },
};
use anyhow::Context;
use askama::Template;
use dtiku_paper::{
    domain::paper::{self, PaperMode},
    model::paper::PaperExtra,
    query::paper::ListPaperQuery as PaperListQuery,
    service::{label::LabelService, paper::PaperService},
};
use spring_sea_orm::pagination::Pagination;
use spring_web::{
    axum::{
        response::{Html, IntoResponse},
        Extension, Form, Json,
    },
    error::{KnownWebError, Result},
    extractor::{Component, Path, Query},
    get, post,
};

#[get("/paper")]
async fn list_paper(
    Component(ps): Component<PaperService>,
    Component(ls): Component<LabelService>,
    Extension(global): Extension<GlobalVariables>,
    Query(query): Query<ListPaperQuery>,
    page: Pagination,
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
            page,
        }
    } else {
        PaperListQuery {
            paper_type: paper_type.id,
            label_id: query.label_id,
            page,
        }
    };
    let label = label_tree.get_label(query.label_id);
    let page = ps.find_paper_by_query(&query).await?;
    Ok(ListPaperTemplate::new(
        global, query, label_tree, paper_type, label, page,
    ))
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
    Form(params): Form<HashMap<String, String>>,
) -> Result<impl IntoResponse> {
    let paper = ps
        .find_paper_by_id(id, PaperMode::Exercise)
        .await?
        .ok_or_else(|| KnownWebError::not_found("试卷未找到"))?;

    let mut user_answer = HashMap::new();
    let mut answer_q_time = HashMap::new();

    for (k, v) in params {
        if let Some(rest_qid) = k.strip_prefix("qt.") {
            if let Ok(qid) = rest_qid.parse::<i32>() {
                if let Ok(time) = v.parse::<u64>() {
                    answer_q_time.insert(qid, time);
                }
            }
        } else if let Ok(qid) = k.parse::<i32>() {
            user_answer.insert(qid, v);
        }
    }
    let paper_model = paper.p.clone();
    let mut t: ChapterPaperTemplate = paper.to_template(global);
    t.report = Some(paper::compute_report(
        &paper_model,
        &t.questions,
        &user_answer,
        &answer_q_time,
    ));
    t.user_answer = Some(user_answer);
    Ok(Html(t.render().context("render failed")?))
}

#[get("/api/paper/{prefix}/title/like")]
async fn paper_title_like(
    Path(prefix): Path<String>,
    Component(ps): Component<PaperService>,
    Query(query): Query<PaperTitleLikeQuery>,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let paper_type = global
        .get_paper_type_by_prefix(&prefix)
        .ok_or_else(|| KnownWebError::bad_request("试卷类型不存在"))?;
    let ps = ps.search_by_name(paper_type.id, &query.title).await?;
    Ok(Json(ps))
}
