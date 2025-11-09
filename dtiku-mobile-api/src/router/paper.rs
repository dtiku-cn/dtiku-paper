use dtiku_paper::{
    domain::paper::PaperMode, model::paper, query::paper::ListPaperQuery,
    service::paper::PaperService,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use spring_sea_orm::pagination::Pagination;
use spring_web::{
    axum::Json,
    error::{KnownWebError, Result},
    extractor::{Component, Path, Query},
    get_api,
};

#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct PaperListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub exam_category_id: Option<i16>,
    pub year: Option<i32>,
    pub province: Option<String>,
    pub keyword: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct PaperResponse {
    pub id: i32,
    pub title: String,
    pub year: i16,
    pub province: Option<String>,
    pub paper_type: i16,
    pub label_id: i32,
    pub mode: i16,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

impl From<paper::Model> for PaperResponse {
    fn from(p: paper::Model) -> Self {
        Self {
            id: p.id,
            title: p.title,
            year: p.year,
            province: None,
            paper_type: p.paper_type,
            label_id: p.label_id,
            mode: 0,
            created_at: chrono::Local::now().naive_local(),
        }
    }
}

/// GET /api/paper/list
#[get_api("/api/paper/list")]
async fn api_paper_list(
    Component(ps): Component<PaperService>,
    Query(q): Query<PaperListQuery>,
) -> Result<Json<PaginatedResponse<PaperResponse>>> {
    let page = q.page.unwrap_or(1);
    let page_size = q.page_size.unwrap_or(20);

    let pagination = Pagination {
        page,
        size: page_size,
    };

    let paper_type = q.exam_category_id.unwrap_or(0);
    let label_id = 0;

    let query = ListPaperQuery {
        paper_type,
        label_id,
        page: pagination,
    };

    let page = ps.find_paper_by_query(&query).await?;

    Ok(Json(PaginatedResponse {
        data: page.content.into_iter().map(PaperResponse::from).collect(),
        total: page.total_elements,
        page: page.page,
        page_size: page.size,
    }))
}

/// GET /api/paper/{id}
#[get_api("/api/paper/{id}")]
async fn api_paper_detail(
    Path(id): Path<i32>,
    Component(ps): Component<PaperService>,
) -> Result<Json<PaperResponse>> {
    let paper = ps
        .find_paper_by_id(id, PaperMode::default())
        .await?
        .ok_or_else(|| KnownWebError::not_found("试卷不存在"))?;

    Ok(Json(PaperResponse::from(paper.p)))
}

/// GET /api/paper/cluster
#[get_api("/api/paper/cluster")]
async fn api_paper_cluster(Query(q): Query<PaperListQuery>) -> Result<Json<serde_json::Value>> {
    let paper_type = q.exam_category_id.unwrap_or(0);

    let cluster_data = serde_json::json!({
        "paper_type": paper_type,
        "cluster": {}
    });

    Ok(Json(cluster_data))
}
