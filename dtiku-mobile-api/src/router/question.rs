use dtiku_paper::{
    domain::question::QuestionSearch,
    model::question::{self, QuestionWithPaper},
    service::question::QuestionService,
};
use serde::{Deserialize, Serialize};
use spring_web::{
    axum::{response::IntoResponse, Json},
    error::{KnownWebError, Result},
    extractor::{Component, Path, Query},
    get_api,
};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct QuestionSearchQuery {
    pub keyword: Option<String>,
    pub question_type: Option<String>,
    pub tags: Option<Vec<String>>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub exam_id: Option<i16>,
}

#[derive(Debug, Deserialize)]
pub struct QuestionRecommendQuery {
    pub limit: Option<usize>,
    pub exclude_ids: Option<Vec<i32>>,
}

#[derive(Debug, Deserialize)]
pub struct QuestionSectionQuery {
    pub section_id: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct QuestionResponse {
    pub id: i32,
    pub content: String,
    pub exam_id: i16,
    pub paper_type: i16,
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

impl From<question::Model> for QuestionResponse {
    fn from(q: question::Model) -> Self {
        Self {
            id: q.id,
            content: q.content,
            exam_id: q.exam_id,
            paper_type: q.paper_type,
        }
    }
}

impl From<QuestionWithPaper> for QuestionResponse {
    fn from(q: QuestionWithPaper) -> Self {
        let (exam_id, paper_type) = q.papers.first().map(|p| (p.paper.exam_id, p.paper.paper_type)).unwrap_or((0, 0));
        Self {
            id: q.id,
            content: q.content,
            exam_id,
            paper_type,
        }
    }
}

/// GET /api/question/search
#[get_api("/api/question/search")]
async fn api_question_search(
    Component(qs): Component<QuestionService>,
    Query(q): Query<QuestionSearchQuery>,
) -> Result<impl IntoResponse> {
    let keyword = q.keyword.unwrap_or_default();
    
    if keyword.is_empty() {
        return Ok(Json(PaginatedResponse {
            data: vec![],
            total: 0,
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        }));
    }

    let search = QuestionSearch {
        content: keyword,
        exam_id: q.exam_id,
        paper_type: None,
    };

    let mut questions = qs.search_question(&search).await?;
    
    let page_size = q.page_size.unwrap_or(20) as usize;
    questions.truncate(page_size);

    let total = questions.len() as u64;
    let page = q.page.unwrap_or(1);
    let page_size = q.page_size.unwrap_or(20);

    Ok(Json(PaginatedResponse {
        data: questions.into_iter().map(QuestionResponse::from).collect(),
        total,
        page,
        page_size,
    }))
}

/// GET /api/question/{id}
#[get_api("/api/question/{id}")]
async fn api_question_detail(
    Path(id): Path<i32>,
    Component(qs): Component<QuestionService>,
) -> Result<impl IntoResponse> {
    let question = qs
        .full_question_by_id(id)
        .await?
        .ok_or_else(|| KnownWebError::not_found("题目不存在"))?;

    Ok(Json(QuestionResponse::from(question)))
}

/// GET /api/question/recommend
#[get_api("/api/question/recommend")]
async fn api_question_recommend(
    Query(q): Query<QuestionRecommendQuery>,
    Component(qs): Component<QuestionService>,
) -> Result<impl IntoResponse> {
    let base_id = q.exclude_ids.as_ref().and_then(|ids| ids.first().copied()).unwrap_or(1);
    
    let mut questions = qs.recommend_question(base_id).await?;

    if let Some(limit) = q.limit {
        questions.truncate(limit);
    }

    if let Some(exclude_ids) = q.exclude_ids {
        questions.retain(|q| !exclude_ids.contains(&q.id));
    }

    Ok(Json(questions.into_iter().map(QuestionResponse::from).collect::<Vec<_>>()))
}

/// GET /api/question/section
#[get_api("/api/question/section")]
async fn api_question_section(
    Query(q): Query<QuestionSectionQuery>,
) -> Result<impl IntoResponse> {
    Ok(Json(serde_json::json!({
        "questions": [],
        "section_id": q.section_id,
    })))
}

