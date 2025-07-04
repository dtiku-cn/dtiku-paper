use crate::{
    query::idiom::IdiomReq,
    views::{
        idiom::{IdiomDetailTemplate, ListIdiomTemplate},
        GlobalVariables,
    },
};
use anyhow::Context;
use askama::Template;
use axum_extra::extract::Query;
use dtiku_paper::{domain::label::LabelTree, service::label::LabelService};
use dtiku_stats::{
    model::sea_orm_active_enums::IdiomType, query::IdiomSearch, service::idiom::IdiomService,
};
use spring_sea_orm::pagination::{Page, Pagination};
use spring_web::{
    axum::{
        response::{Html, IntoResponse},
        Extension, Json,
    },
    error::{KnownWebError, Result},
    extractor::{Component, Path},
    get, routes,
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
        model: IdiomType::Idiom,
        label_tree,
        req,
        page: Page::new(vec![], &page, 0),
    };
    Ok(Html(t.render().context("render failed")?))
}

#[get("/idiom/like")]
async fn idiom_like(
    Component(is): Component<IdiomService>,
    Query(search): Query<IdiomSearch>,
) -> Result<impl IntoResponse> {
    Ok(Json(is.search_idiom(search).await?))
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
        model: IdiomType::Word,
        label_tree,
        req,
        page: Page::new(vec![], &page, 0),
    };
    Ok(Html(t.render().context("render failed")?))
}

#[routes]
#[get("/word/{idiom_id}")]
#[get("/idiom/{idiom_id}")]
async fn idiom_detail(
    Component(is): Component<IdiomService>,
    Path(idiom_id): Path<i32>,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let idiom = is
        .get_idiom_detail(idiom_id)
        .await?
        .ok_or_else(|| KnownWebError::not_found("成语未找到"))?;

    let t = IdiomDetailTemplate { global, idiom };
    Ok(Html(t.render().context("render failed")?))
}
