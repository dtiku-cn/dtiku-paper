use crate::model::{
    self,
    question::{Column, QuestionExtra},
};
use sea_orm::{sea_query::IntoCondition, ColumnTrait};
use serde::Deserialize;

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

pub struct FullQuestion {
    pub id: i32,
    pub content: String,
    pub extra: QuestionExtra,
    pub num: usize,
    pub materials: Option<Vec<model::material::Material>>,
    pub solutions: Option<Vec<model::solution::Model>>,
    pub chapter: Option<model::paper::PaperChapter>,
}

impl FullQuestion {
    pub fn new(
        materials: Option<Vec<model::material::Material>>,
        solutions: Option<Vec<model::solution::Model>>,
        chapter: Option<model::paper::PaperChapter>,
        q: model::question::Question,
    ) -> Self {
        Self {
            id: q.id,
            content: q.content,
            extra: q.extra,
            num: q.num as usize,
            materials,
            solutions,
            chapter,
        }
    }

    pub fn option_len(&self) -> usize {
        self.extra.option_len()
    }

    pub fn get_answer(&self) -> Option<String> {
        match &self.solutions {
            None => None,
            Some(ss) => ss.first().and_then(|s| s.extra.get_answer()),
        }
    }

    pub fn is_answer(&self, index0: &usize) -> bool {
        match &self.solutions {
            None => false,
            Some(ss) => ss
                .first()
                .map(|s| s.extra.is_answer(*index0))
                .unwrap_or_default(),
        }
    }
}
