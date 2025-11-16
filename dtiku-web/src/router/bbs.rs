use super::Claims;
use crate::{
    query::bbs::{IssueDetailReq, IssueReq},
    router::error_messages,
    service::issue::IssueService,
    views::{
        bbs::{IssueEditorTemplate, IssueTemplate, ListIssueTemplate, PaywallContentTemplate},
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
    // 判断用户是否有权限查看完整内容
    let has_access = global.user.as_ref()
        .map(|u| !u.is_expired())
        .unwrap_or(false);
    
    let html = if req.html {
        // 对于AJAX请求返回的HTML片段，也需要检查权限
        let mut html = is.find_issue_html_by_id(id)
            .await?
            .ok_or_else(|| KnownWebError::not_found(error_messages::ISSUE_NOT_FOUND))?;
        
        // 如果没有访问权限，截断HTML内容
        if !has_access {
            let is_logged_in = global.user.is_some();
            html = truncate_html_content(&html, is_logged_in)?;
        }
        
        html
    } else {
        let mut issue = is
            .find_issue_by_id(id)
            .await?
            .ok_or_else(|| KnownWebError::not_found(error_messages::ISSUE_NOT_FOUND))?;
        
        // 如果没有访问权限，截断HTML内容
        if !has_access {
            issue.truncate_html();
        }
        
        let temp = IssueTemplate { global, issue };
        temp.render().context("render failed")?
    };
    Ok(Html(html))
}

/// 截断HTML内容的辅助函数，并添加付费墙UI
fn truncate_html_content(html: &str, is_logged_in: bool) -> Result<String> {
    use crate::views::bbs::truncate_html_by_text_length;
    const PREVIEW_TEXT_LENGTH: usize = 300;
    
    // 先检查文本长度是否需要截断
    use scraper::Html;
    let fragment = Html::parse_fragment(html);
    let full_text: String = fragment.root_element().text().collect();
    
    if full_text.chars().count() <= PREVIEW_TEXT_LENGTH {
        return Ok(html.to_string());
    }
    
    let truncated = truncate_html_by_text_length(html, PREVIEW_TEXT_LENGTH);
    
    // 使用模板渲染付费墙内容
    let template = PaywallContentTemplate {
        content: truncated,
        is_logged_in,
    };
    
    Ok(template.render().context("render paywall template failed")?)
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
