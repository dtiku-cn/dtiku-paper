use sea_orm::{sea_query::IntoCondition, ColumnTrait};
use serde::Deserialize;
use spring_sea_orm::pagination::Pagination;

use crate::model::idiom_ref_stats;

#[derive(Debug, Clone, Deserialize)]
pub struct IdiomQuery {
    #[serde(default, rename = "lid")]
    pub label_id: Vec<i32>,
    #[serde(flatten)]
    pub page: Pagination,
}

impl IntoCondition for IdiomQuery {
    fn into_condition(self) -> sea_orm::sea_query::Condition {
        let mut condition = sea_orm::sea_query::Condition::all();
        if !self.label_id.is_empty() {
            condition = condition.add(idiom_ref_stats::Column::LabelId.is_in(self.label_id));
        }
        condition
    }
}
