use crate::{
    query::idiom::IdiomReq,
    views::{idiom::ListIdiomTemplate, GlobalVariables},
};
use anyhow::Context;
use askama::Template;
use axum_extra::extract::Query;
use dtiku_paper::{domain::label::LabelTree, service::label::LabelService};
use dtiku_stats::StatsModelType;
use spring_sea_orm::pagination::{Page, Pagination};
use spring_web::{
    axum::{
        response::{Html, IntoResponse},
        Extension,
    },
    error::Result,
    extractor::Component,
    get,
};

#[get("/idiom")]
async fn list_idiom(
    Component(ls): Component<LabelService>,
    Query(req): Query<IdiomReq>,
    page: Pagination,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let label_tree = match global.get_paper_type_by_prefix("xingce") {
        Some(paper_type) => ls.find_all_label_by_paper_type(paper_type.id).await?,
        None => LabelTree::none(),
    };

    let t = ListIdiomTemplate {
        global,
        model: StatsModelType::Idiom,
        label_tree,
        req,
        page: Page::new(vec![], &page, 0),
    };
    Ok(Html(t.render().context("render failed")?))
}

#[get("/word")]
async fn list_word(
    Component(ls): Component<LabelService>,
    Query(req): Query<IdiomReq>,
    page: Pagination,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let label_tree = match global.get_paper_type_by_prefix("xingce") {
        Some(paper_type) => ls.find_all_label_by_paper_type(paper_type.id).await?,
        None => LabelTree::none(),
    };
    let t = ListIdiomTemplate {
        global,
        model: StatsModelType::Word,
        label_tree,
        req,
        page: Page::new(vec![], &page, 0),
    };
    Ok(Html(t.render().context("render failed")?))
}
