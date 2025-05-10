use crate::model::label::Column;
use sea_orm::{sea_query::IntoCondition, ColumnTrait};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct LabelQuery {
    pub(crate) pid: i32,
    pub(crate) paper_type: i16,
}

impl IntoCondition for LabelQuery {
    fn into_condition(self) -> sea_orm::Condition {
        Column::Pid
            .eq(self.pid)
            .and(Column::PaperType.eq(self.paper_type))
            .into_condition()
    }
}
