use serde::Deserialize;
use spring_sea_orm::pagination::Pagination;

#[derive(Debug, Deserialize)]
pub struct ListPaperQuery {
    #[serde(rename = "ty")]
    pub paper_type_prefix: String,
    #[serde(default, rename = "lid")]
    pub label_id: i32,
    #[serde(flatten)]
    pub page: Pagination,
}
