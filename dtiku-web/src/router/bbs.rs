use super::Claims;
use crate::{
    query::bbs::{IssueDetailReq, IssueReq},
    router::error_messages,
    service::issue::IssueService,
    views::{
        bbs::{FullIssue, IssueEditorTemplate, IssueTemplate, ListIssueTemplate, PaywallContentTemplate},
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
    let html = if req.html {
        // AJAX请求：返回HTML片段
        let (html, author_id) = is
            .find_issue_html_with_author(id)
            .await?
            .ok_or_else(|| KnownWebError::not_found(error_messages::ISSUE_NOT_FOUND))?;
        
        let access_control = AccessControl::from_global(&global, author_id);
        access_control.apply_paywall_to_html(&html)?
    } else {
        // 完整页面请求
        let mut issue = is
            .find_issue_by_id(id)
            .await?
            .ok_or_else(|| KnownWebError::not_found(error_messages::ISSUE_NOT_FOUND))?;
        
        let access_control = AccessControl::from_global(&global, issue.user_id);
        access_control.apply_paywall_to_issue(&mut issue);
        
        IssueTemplate { global, issue }
            .render()
            .context("render failed")?
    };
    
    Ok(Html(html))
}

/// 访问控制辅助结构
struct AccessControl {
    has_access: bool,
    is_logged_in: bool,
}

impl AccessControl {
    /// 从全局变量创建访问控制
    /// author_id: 帖子作者的用户ID
    fn from_global(global: &GlobalVariables, author_id: i32) -> Self {
        let is_logged_in = global.user.is_some();
        
        // 检查用户是否有访问权限：
        // 1. 用户已付费（未过期）
        // 2. 或者当前用户就是帖子作者
        let has_access = global.user.as_ref()
            .map(|u| !u.is_expired() || u.id == author_id)
            .unwrap_or(false);
        
        Self { has_access, is_logged_in }
    }
    
    /// 对HTML内容应用付费墙（用于AJAX请求）
    fn apply_paywall_to_html(&self, html: &str) -> Result<String> {
        if self.has_access {
            return Ok(html.to_string());
        }
        
        apply_html_paywall(html, self.is_logged_in)
    }
    
    /// 对Issue对象应用付费墙（用于完整页面）
    fn apply_paywall_to_issue(&self, issue: &mut FullIssue) {
        if !self.has_access {
            issue.truncate_html();
        }
    }
}

/// 对HTML应用付费墙：截断内容并添加付费墙UI
fn apply_html_paywall(html: &str, is_logged_in: bool) -> Result<String> {
    use crate::views::bbs::truncate_html_by_text_length;
    use scraper::Html;
    
    const PREVIEW_TEXT_LENGTH: usize = 300;
    
    // 检查是否需要截断
    let fragment = Html::parse_fragment(html);
    let full_text: String = fragment.root_element().text().collect();
    
    if full_text.chars().count() <= PREVIEW_TEXT_LENGTH {
        return Ok(html.to_string());
    }
    
    // 截断并渲染付费墙UI
    let truncated = truncate_html_by_text_length(html, PREVIEW_TEXT_LENGTH);
    
    Ok(PaywallContentTemplate {
        content: truncated,
        is_logged_in,
    }
    .render()
    .context("render paywall template failed")?)
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
