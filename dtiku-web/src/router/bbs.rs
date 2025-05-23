use crate::{
    service::issue::IssueService,
    views::{
        bbs::{IssueTemplate, ListIssueTemplate},
        GlobalVariables,
    },
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
    error::{KnownWebError, Result},
    extractor::{Component, Path, Query},
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

#[get("/bbs/issue/{id}")]
async fn issue(
    Component(is): Component<IssueService>,
    Path(id): Path<i32>,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let issue = is
        .find_issue_by_id(id)
        .await?
        .ok_or_else(|| KnownWebError::not_found("issue not found"))?;

    let t = IssueTemplate { global, issue };
    Ok(Html(t.render().context("render failed")?))
}
