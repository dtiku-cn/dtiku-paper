use crate::views::GlobalVariables;
use dtiku_stats::{
    model::sea_orm_active_enums::IdiomType,
    query::{IdiomQuery, IdiomSearch},
    service::idiom::IdiomService,
};
use serde::{Deserialize, Serialize};
use spring_sea_orm::pagination::Pagination;
use spring_web::{
    axum::{response::IntoResponse, Extension, Json},
    error::{KnownWebError, Result},
    extractor::{Component, Path, Query},
    get,
};

#[derive(Debug, Deserialize)]
pub struct IdiomListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub keyword: Option<String>,
    pub initial: Option<String>,
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
#[get("/api/idiom/list")]
async fn api_idiom_list(
    Component(is): Component<IdiomService>,
    Extension(global): Extension<GlobalVariables>,
    Query(q): Query<IdiomListQuery>,
) -> Result<impl IntoResponse> {
    let page = q.page.unwrap_or(1);
    let page_size = q.page_size.unwrap_or(20);

    let pagination = Pagination { page, size: page_size };

    let paper_type = global
        .get_paper_type_by_prefix("xingce")
        .map(|pt| pt.id)
        .unwrap_or(0);

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

    // 转换为响应格式
    let data: Vec<IdiomResponse> = page_result
        .content
        .into_iter()
        .map(|item| IdiomResponse {
            id: item.idiom_id,
            text: item.text,
            pinyin: Some(item.baobian.clone()), // 使用 baobian 作为拼音
            explanation: Some(item.explain.clone()),
            source: None, // IdiomStats 没有 source 字段
            example: None, // IdiomStats 没有 example 字段
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
#[get("/api/idiom/{id}")]
async fn api_idiom_detail(
    Path(_id): Path<i32>,
) -> Result<Json<IdiomResponse>> {
    // 由于现有的 API 使用 text 作为查询，而移动端使用 id
    // 这里需要根据 ID 查询，暂时返回错误
    // 实际应该扩展 IdiomService 支持通过 ID 查询
    Err(KnownWebError::not_found("成语查询暂不支持 ID，请使用 text 参数").into())
}

