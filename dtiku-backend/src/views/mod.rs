use serde::Serialize;

pub mod config;
pub mod exam;
pub mod task;
pub mod test;

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
