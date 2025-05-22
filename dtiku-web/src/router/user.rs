use crate::views::{bbs::ListIssueTemplate, GlobalVariables};
use anyhow::Context;
use askama::Template;
use dtiku_bbs::{model::IssueQuery, service::issue::IssueService};
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

#[get("/api/user")]
async fn user_detail(
    Query(query): Query<OAuthQuery>,
    Component(auth): Component<AuthService>,
) -> Result<impl IntoResponse> {
    todo!()
}