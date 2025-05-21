use crate::views::{bbs::ListIssueTemplate, GlobalVariables};
use anyhow::Context;
use askama::Template;
use dtiku_bbs::model::{Issue, IssueQuery};
use spring_sea_orm::{pagination::Pagination, DbConn};
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
    Component(db): Component<DbConn>,
    Query(query): Query<IssueQuery>,
    pagination: Pagination,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let page = Issue::search(&db, &query, &pagination).await?;
    let t = ListIssueTemplate {
        global,
        page,
        query,
    };
    Ok(Html(t.render().context("render failed")?))
}
