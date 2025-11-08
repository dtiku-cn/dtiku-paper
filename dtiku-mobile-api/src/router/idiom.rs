use dtiku_stats::{
    model::sea_orm_active_enums::IdiomType,
    query::{IdiomQuery, IdiomSearch},
    service::idiom::IdiomService,
};
use serde::{Deserialize, Serialize};
use spring_sea_orm::pagination::Pagination;
use spring_web::{
    axum::{response::IntoResponse, Json},
    error::{KnownWebError, Result},
    extractor::{Component, Path, Query},
    get_api,
};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct IdiomListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub keyword: Option<String>,
    pub initial: Option<String>,
    pub paper_type: Option<i16>,
}

#[derive(Debug, Serialize)]
pub struct IdiomResponse {
    pub id: i32,
    pub text: String,
    pub pinyin: Option<String>,
    pub explanation: Option<String>,
    pub source: Option<String>,
    pub example: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

/// GET /api/idiom/list
#[get_api("/api/idiom/list")]
async fn api_idiom_list(
    Component(is): Component<IdiomService>,
    Query(q): Query<IdiomListQuery>,
) -> Result<impl IntoResponse> {
    let page = q.page.unwrap_or(1);
    let page_size = q.page_size.unwrap_or(20);

    let pagination = Pagination { page, size: page_size };
    let paper_type = q.paper_type.unwrap_or(0);

    let page_result = if let Some(keyword) = q.keyword {
        let search = IdiomSearch {
            ty: IdiomType::Idiom,
            text: keyword,
        };
        is.search_idiom_stats(&search, paper_type, vec![], &pagination)
            .await?
    } else {
        let query = IdiomQuery {
            label_id: vec![],
            page: pagination,
        };
        is.get_idiom_stats(IdiomType::Idiom, paper_type, &query)
            .await?
    };

    let data: Vec<IdiomResponse> = page_result
        .content
        .into_iter()
        .map(|item| IdiomResponse {
            id: item.idiom_id,
            text: item.text,
            pinyin: Some(item.baobian.clone()),
            explanation: Some(item.explain.clone()),
            source: None,
            example: None,
        })
        .collect();

    Ok(Json(PaginatedResponse {
        data,
        total: page_result.total_elements,
        page: page_result.page,
        page_size: page_result.size,
    }))
}

/// GET /api/idiom/{id}
#[get_api("/api/idiom/{id}")]
async fn api_idiom_detail(
    Path(_id): Path<i32>,
) -> Result<Json<IdiomResponse>> {
    Err(KnownWebError::not_found("成语查询暂不支持 ID，请使用 text 参数").into())
}

