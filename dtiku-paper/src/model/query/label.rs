use crate::model::label::Column;
use sea_orm::{sea_query::IntoCondition, ColumnTrait};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct LabelQuery {
    pub pid: i32,
    pub paper_type: i16,
    pub hidden: Option<bool>,
}

impl IntoCondition for LabelQuery {
    fn into_condition(self) -> sea_orm::Condition {
        let filter = Column::Pid
            .eq(self.pid)
            .and(Column::PaperType.eq(self.paper_type));
        if let Some(hidden) = self.hidden {
            filter.and(Column::Hidden.eq(hidden)).into_condition()
        } else {
            filter.into_condition()
        }
    }
}
