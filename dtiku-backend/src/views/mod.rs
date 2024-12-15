use serde::{Deserialize, Serialize};

pub mod config;
pub mod task;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetListParams<T> {
    pub filter: T,
    pub pagination: Pagination,
    pub sort: Option<Sort>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}

fn default_page() -> i64 {
    0
}

fn default_per_page() -> i64 {
    10
}

#[derive(Debug, Deserialize)]
pub struct Sort {
    pub field: String,
    pub order: Order,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Order {
    Asc,
    Desc,
}

#[derive(Debug, Serialize)]
pub struct GetListResult<T> {
    data: Vec<T>,
    total: i64,
}

impl<T> GetListResult<T> {
    pub fn new(data: Vec<T>, total: i64) -> Self {
        Self { total, data }
    }
}

impl<T> From<Vec<T>> for GetListResult<T> {
    fn from(data: Vec<T>) -> Self {
        let total = data.len() as i64;
        Self::new(data, total)
    }
}
