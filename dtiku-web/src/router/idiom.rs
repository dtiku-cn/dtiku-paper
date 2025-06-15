use crate::{
    query::idiom::IdiomReq,
    views::{idiom::ListIdiomTemplate, GlobalVariables},
};
use anyhow::Context;
use askama::Template;
use dtiku_stats::StatsModelType;
use spring_sea_orm::pagination::Pagination;
use spring_web::{
    axum::{
        response::{Html, IntoResponse},
        Extension,
    },
    error::Result,
    extractor::Query,
    get,
};

#[get("/idiom")]
async fn list_idiom(
    Query(req): Query<IdiomReq>,
    page: Pagination,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let t = ListIdiomTemplate {
        global,
        model: StatsModelType::Idiom,
        req,
        page,
    };
    Ok(Html(t.render().context("render failed")?))
}

#[get("/word")]
async fn list_word(
    Query(req): Query<IdiomReq>,
    page: Pagination,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let t = ListIdiomTemplate {
        global,
        model: StatsModelType::Word,
        req,
        page,
    };
    Ok(Html(t.render().context("render failed")?))
}
