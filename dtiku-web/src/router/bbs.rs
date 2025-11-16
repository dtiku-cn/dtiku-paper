use super::Claims;
use crate::{
    query::bbs::{IssueDetailReq, IssueReq},
    router::error_messages,
    service::issue::IssueService,
    views::{
        bbs::{IssueContentTemplate, IssueEditorTemplate, IssueTemplate, ListIssueTemplate},
        GlobalVariables,
    },
};
use anyhow::Context;
use askama::Template;
use dtiku_bbs::model::{issue, Issue, IssueQuery};
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ActiveValue::Unchanged};
use spring_sea_orm::{pagination::Pagination, DbConn};
use spring_web::{
    axum::{
        response::{Html, IntoResponse, Redirect},
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
    Component(db): Component<DbConn>,
    pagination: Pagination,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let page = is.search(&query, &pagination).await?;
    let pin_issues = Issue::find_pins_by_topic(&db, query.topic).await?;
    Ok(ListIssueTemplate {
        global,
        page,
        query,
        pin_issues,
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
        .ok_or_else(|| KnownWebError::not_found(error_messages::ISSUE_NOT_FOUND))?;
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
        paid: Set(req.paid),
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
    Query(req): Query<IssueDetailReq>,
) -> Result<impl IntoResponse> {
    let issue = is
        .find_issue_by_id(id)
        .await?
        .ok_or_else(|| KnownWebError::not_found(error_messages::ISSUE_NOT_FOUND))?;
    
    let html = if req.html {
        // AJAX请求：返回HTML片段，使用简单模板渲染付费墙逻辑
        IssueContentTemplate { global, issue }
            .render()
            .context("render issue content failed")?
    } else {
        // 完整页面请求，让模板控制付费墙逻辑
        IssueTemplate { global, issue }
            .render()
            .context("render failed")?
    };
    
    Ok(Html(html))
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
        paid: Set(req.paid),
        ..Default::default()
    }
    .update(&db)
    .await
    .context("更新失败")?;
    Ok(Redirect::to(&format!("/bbs/issue/{}", m.id)))
}
