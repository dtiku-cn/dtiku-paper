use crate::{router::Claims, service::issue::IssueService, views::bbs::FullIssue};
use anyhow::Context;
use dtiku_bbs::model::{issue, IssueQuery, TopicType};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ActiveValue::Unchanged, DbConn, EntityTrait,
};
use serde::{Deserialize, Serialize};
use spring_sea_orm::pagination::Pagination;
use spring_web::{
    axum::{response::IntoResponse, Json},
    error::{KnownWebError, Result},
    extractor::{Component, Path, Query},
    delete, get, post, put,
};

#[derive(Debug, Deserialize)]
pub struct IssueListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub category: Option<String>,
    pub sort: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct IssueCreateRequest {
    pub title: String,
    pub content: String,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct IssueUpdateRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct IssueResponse {
    pub id: i32,
    pub title: String,
    pub content: String,
    pub user_id: i32,
    pub created: chrono::NaiveDateTime,
    pub modified: chrono::NaiveDateTime,
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

impl From<FullIssue> for IssueResponse {
    fn from(i: FullIssue) -> Self {
        Self {
            id: i.id,
            title: i.title,
            content: i.markdown.clone(),
            user_id: i.user_id,
            created: i.created,
            modified: i.modified,
        }
    }
}

impl From<issue::Model> for IssueResponse {
    fn from(i: issue::Model) -> Self {
        Self {
            id: i.id,
            title: i.title,
            content: i.markdown.clone(),
            user_id: i.user_id,
            created: i.created,
            modified: i.modified,
        }
    }
}

/// GET /api/issue/list
#[get("/api/issue/list")]
async fn api_issue_list(
    Component(is): Component<IssueService>,
    Query(q): Query<IssueListQuery>,
) -> Result<impl IntoResponse> {
    let page = q.page.unwrap_or(1);
    let page_size = q.page_size.unwrap_or(20);

    let pagination = Pagination { page, size: page_size };

    let query = IssueQuery {
        title: None,
        topic: None,
    };
    let page_result = is.search(&query, &pagination).await?;

    Ok(Json(PaginatedResponse {
        data: page_result.content.into_iter().map(IssueResponse::from).collect(),
        total: page_result.total_elements,
        page: page_result.page,
        page_size: page_result.size,
    }))
}

/// GET /api/issue/{id}
#[get("/api/issue/{id}")]
async fn api_issue_detail(
    Path(id): Path<i32>,
    Component(is): Component<IssueService>,
) -> Result<impl IntoResponse> {
    let issue = is
        .find_issue_by_id(id)
        .await?
        .ok_or_else(|| KnownWebError::not_found("帖子不存在"))?;

    Ok(Json(IssueResponse::from(issue)))
}

/// POST /api/issue/create
#[post("/api/issue/create")]
async fn api_issue_create(
    claims: Claims,
    Component(db): Component<DbConn>,
    Json(req): Json<IssueCreateRequest>,
) -> Result<impl IntoResponse> {
    // 简单的 markdown 到 html 转换（实际项目应该使用专业的 markdown 解析器）
    let html = req.content.clone();

    let new_issue = issue::ActiveModel {
        title: Set(req.title),
        markdown: Set(req.content),
        html: Set(html),
        user_id: Set(claims.user_id),
        topic: Set(TopicType::Xingce), // 默认使用行测主题
        ..Default::default()
    };

    let issue = new_issue.insert(&db).await.context("创建帖子失败")?;

    Ok(Json(IssueResponse::from(issue)))
}

/// PUT /api/issue/{id}/update
#[put("/api/issue/{id}/update")]
async fn api_issue_update(
    claims: Claims,
    Path(id): Path<i32>,
    Component(db): Component<DbConn>,
    Json(req): Json<IssueUpdateRequest>,
) -> Result<impl IntoResponse> {
    // 检查帖子是否存在并且属于当前用户
    let existing = issue::Entity::find_by_id(id)
        .one(&db)
        .await
        .context("查询帖子失败")?
        .ok_or_else(|| KnownWebError::not_found("帖子不存在"))?;

    if existing.user_id != claims.user_id {
        return Err(KnownWebError::forbidden("无权修改此帖子"))?;
    }

    let mut update_model = issue::ActiveModel {
        id: Unchanged(id),
        user_id: Unchanged(claims.user_id),
        ..Default::default()
    };

    if let Some(title) = req.title {
        update_model.title = Set(title);
    }

    if let Some(content) = req.content {
        let html = content.clone();
        update_model.markdown = Set(content);
        update_model.html = Set(html);
    }

    let issue = update_model.update(&db).await.context("更新帖子失败")?;

    Ok(Json(IssueResponse::from(issue)))
}

/// DELETE /api/issue/{id}/delete
#[delete("/api/issue/{id}/delete")]
async fn api_issue_delete(
    claims: Claims,
    Path(id): Path<i32>,
    Component(db): Component<DbConn>,
) -> Result<impl IntoResponse> {
    // 检查帖子是否存在并且属于当前用户
    let existing = issue::Entity::find_by_id(id)
        .one(&db)
        .await
        .context("查询帖子失败")?
        .ok_or_else(|| KnownWebError::not_found("帖子不存在"))?;

    if existing.user_id != claims.user_id {
        return Err(KnownWebError::forbidden("无权删除此帖子"))?;
    }

    issue::Entity::delete_by_id(id)
        .exec(&db)
        .await
        .context("删除帖子失败")?;

    Ok(Json(serde_json::json!({"success": true})))
}

