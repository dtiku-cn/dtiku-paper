use super::Claims;
use crate::{
    query::bbs::IssueReq,
    service::issue::IssueService,
    views::{
        bbs::{IssueEditorTemplate, IssueTemplate, ListIssueTemplate},
        GlobalVariables,
    },
};
use anyhow::Context;
use dtiku_bbs::model::{issue, IssueQuery};
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ActiveValue::Unchanged};
use spring_sea_orm::{pagination::Pagination, DbConn};
use spring_web::{
    axum::{
        response::{IntoResponse, Redirect},
        Extension, Form,
    },
    error::{KnownWebError, Result},
    extractor::{Component, Path, Query},
    get, post,
};

#[get("/bbs")]
async fn list_issue(
    Component(is): Component<IssueService>,
    Query(query): Query<IssueQuery>,
    pagination: Pagination,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let page = is.search(&query, &pagination).await?;
    Ok(ListIssueTemplate {
        global,
        page,
        query,
    })
}

#[get("/bbs/issue")]
async fn new_issue(Extension(global): Extension<GlobalVariables>) -> impl IntoResponse {
    IssueEditorTemplate {
        global,
        issue: None,
    }
}

#[get("/bbs/issue/{id}/edit")]
async fn edit_issue(
    Path(id): Path<i32>,
    Component(is): Component<IssueService>,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let issue = is
        .find_issue_by_id(id)
        .await?
        .ok_or_else(|| KnownWebError::not_found("没找到帖子"))?;
    Ok(IssueEditorTemplate {
        global,
        issue: Some(issue),
    })
}

#[post("/bbs/issue")]
async fn submit_issue(
    claims: Claims,
    Component(db): Component<DbConn>,
    Form(req): Form<IssueReq>,
) -> Result<impl IntoResponse> {
    let m = issue::ActiveModel {
        topic: Set(req.topic),
        title: Set(req.title),
        markdown: Set(req.markdown),
        html: Set(req.html),
        user_id: Set(claims.user_id),
        ..Default::default()
    }
    .insert(&db)
    .await
    .context("insert issue failed")?;
    Ok(Redirect::to(&format!("/bbs/issue/{}", m.id)))
}

#[get("/bbs/issue/{id}")]
async fn issue_detail(
    Component(is): Component<IssueService>,
    Path(id): Path<i32>,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let issue = is
        .find_issue_by_id(id)
        .await?
        .ok_or_else(|| KnownWebError::not_found("没找到帖子"))?;

    Ok(IssueTemplate { global, issue })
}

#[post("/bbs/issue/{id}")]
async fn update_issue(
    claims: Claims,
    Component(db): Component<DbConn>,
    Path(id): Path<i32>,
    Form(req): Form<IssueReq>,
) -> Result<impl IntoResponse> {
    let m = issue::ActiveModel {
        id: Unchanged(id),
        topic: Set(req.topic),
        title: Set(req.title),
        markdown: Set(req.markdown),
        html: Set(req.html),
        user_id: Unchanged(claims.user_id),
        ..Default::default()
    }
    .update(&db)
    .await
    .context("更新失败")?;
    Ok(Redirect::to(&format!("/bbs/issue/{}", m.id)))
}
