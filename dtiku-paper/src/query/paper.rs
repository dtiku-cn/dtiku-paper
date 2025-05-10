use serde::Deserialize;
use spring_sea_orm::pagination::Pagination;

#[derive(Debug, Deserialize)]
pub struct ListPaperQuery {
    #[serde(rename = "ty")]
    pub paper_type: i16,
    #[serde(default, rename = "lid")]
    pub label_id: i32,
    #[serde(flatten)]
    pub page: Pagination,
}
