use sea_orm::{sea_query::IntoCondition, ColumnTrait};
use serde::Deserialize;

use crate::model::question::Column;

#[derive(Debug, Clone, Deserialize)]
pub struct QuestionSearch {
    #[serde(default)]
    pub content: String,
    pub exam_id: Option<i16>,
    #[serde(rename = "type")]
    pub paper_type: Option<i16>,
}

impl IntoCondition for QuestionSearch {
    fn into_condition(self) -> sea_orm::Condition {
        let q = Column::ExamId
            .eq(self.exam_id)
            .and(Column::Content.contains(self.content));
        if let Some(paper_type) = self.paper_type {
            q.and(Column::PaperType.eq(paper_type)).into_condition()
        } else {
            q.into_condition()
        }
    }
}
