use crate::model::idiom;
use sea_orm::FromQueryResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdiomStats {
    pub text: String,
    pub idiom_id: i32,
    pub question_count: i64,
    pub paper_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromQueryResult)]
pub struct IdiomRefStatsWithoutLabel {
    pub idiom_id: i32,
    pub question_count: i64,
    pub paper_count: i64,
}

impl IdiomRefStatsWithoutLabel {
    pub fn with_idiom(self, id_text_map: &HashMap<i32, String>) -> IdiomStats {
        IdiomStats {
            text: id_text_map.get(&self.idiom_id).cloned().unwrap_or_default(),
            idiom_id: self.idiom_id,
            question_count: self.question_count,
            paper_count: self.paper_count,
        }
    }
}

impl IdiomStats {
    pub fn from(stats: Option<&IdiomRefStatsWithoutLabel>, m: idiom::Model) -> Self {
        Self {
            text: m.text,
            idiom_id: m.id,
            question_count: stats.map(|s| s.question_count).unwrap_or_default(),
            paper_count: stats.map(|s| s.paper_count).unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdiomDetail {
    pub detail: idiom::Model,
    pub refs: Vec<PaperQuestionRef>,
}

impl IdiomDetail {
    pub(crate) fn new(detail: idiom::Model, refs: Vec<crate::model::idiom_ref::Model>) -> Self {
        Self {
            detail,
            refs: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperQuestionRef {}
