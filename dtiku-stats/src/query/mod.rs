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

impl IdiomQuery {
    pub fn into_condition(&self, ty: IdiomType) -> sea_orm::sea_query::Condition {
        let mut condition = idiom_ref_stats::Column::Ty.eq(ty);
        if !self.label_id.is_empty() {
            condition =
                condition.and(idiom_ref_stats::Column::LabelId.is_in(self.label_id.clone()));
        }
        condition.into_condition()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IdiomSearch {
    pub ty: IdiomType,
    #[serde(default, rename = "q")]
    pub text: String,
}
