use crate::{
    service::issue::IssueService,
    views::{bbs::ListIssueTemplate, GlobalVariables},
};
use anyhow::Context;
use askama::Template;
use dtiku_bbs::model::IssueQuery;
use spring_sea_orm::pagination::Pagination;
use spring_web::{
    axum::{
        response::{Html, IntoResponse},
        Extension,
    },
    error::Result,
    extractor::{Component, Query},
    get,
};

#[get("/bbs")]
async fn list_issue(
    Component(is): Component<IssueService>,
    Query(query): Query<IssueQuery>,
    pagination: Pagination,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let page = is.search(&query, &pagination).await?;
    let t = ListIssueTemplate {
        global,
        page,
        query,
    };
    Ok(Html(t.render().context("render failed")?))
}
