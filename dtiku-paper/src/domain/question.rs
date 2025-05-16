use sea_orm::{sea_query::IntoCondition, ColumnTrait};
use serde::Deserialize;

use crate::model::question::Column;

#[derive(Debug, Clone, Deserialize)]
pub struct QuestionSearch {
    pub content: Option<String>,
    pub exam_id: Option<i16>,
    pub paper_type: Option<i16>,
}

impl IntoCondition for QuestionSearch {
    fn into_condition(self) -> sea_orm::Condition {
        let mut q = Column::ExamId.eq(self.exam_id);
        if let Some(content) = self.content {
            q = q.and(Column::Content.contains(content));
        }
        if let Some(paper_type) = self.paper_type {
            q.and(Column::PaperType.eq(paper_type)).into_condition()
        } else {
            q.into_condition()
        }
    }
}
