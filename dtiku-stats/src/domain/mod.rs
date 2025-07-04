use crate::model::{idiom, idiom_ref_stats, sea_orm_active_enums::IdiomType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdiomStats {
    pub text: String,
    pub ty: IdiomType,
    pub label_id: i32,
    pub idiom_id: i32,
    pub question_count: i64,
    pub paper_count: i64,
}

impl IdiomStats {
    pub fn from(idiom: Option<&String>, stats: idiom_ref_stats::Model) -> Self {
        Self {
            text: idiom.cloned().unwrap_or_default(),
            ty: stats.ty,
            label_id: stats.label_id,
            idiom_id: stats.idiom_id,
            question_count: stats.question_count,
            paper_count: stats.paper_count,
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
