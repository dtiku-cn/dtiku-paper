use sea_orm::{sea_query::IntoCondition, ColumnTrait};
use serde::Deserialize;
use spring_sea_orm::pagination::Pagination;

use crate::model::{idiom_ref_stats, sea_orm_active_enums::IdiomType};

#[derive(Debug, Clone, Deserialize)]
pub struct IdiomQuery {
    #[serde(default, rename = "lid")]
    pub label_id: Vec<i32>,
    #[serde(flatten)]
    pub page: Pagination,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IdiomSearch {
    #[serde(rename = "type")]
    pub ty: IdiomType,
    #[serde(default, rename = "q")]
    pub text: String,
}
