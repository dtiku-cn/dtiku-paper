use crate::model::idiom::{self, BriefIdiom};
use itertools::Itertools;
use sea_orm::FromQueryResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdiomStats {
    pub text: String,
    pub explain: String,
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
    pub fn with_idiom(self, id_text_map: &HashMap<i32, BriefIdiom>) -> IdiomStats {
        IdiomStats {
            text: id_text_map
                .get(&self.idiom_id)
                .map(|i| i.text.clone())
                .unwrap_or_default(),
            explain: id_text_map
                .get(&self.idiom_id)
                .map(|i| i.explain.clone())
                .unwrap_or_default(),
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
            explain: m.explain,
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
    pub jyc: Vec<BriefIdiom>,
    pub fyc: Vec<BriefIdiom>,
}

impl IdiomDetail {
    pub(crate) fn new(
        detail: idiom::Model,
        refs: Vec<crate::model::idiom_ref::Model>,
        jyc: Vec<BriefIdiom>,
        fyc: Vec<BriefIdiom>,
    ) -> Self {
        Self {
            detail,
            refs: vec![],
            jyc,
            fyc,
        }
    }

    pub fn other_jyc(&self) -> Vec<&String> {
        let jyc = self.jyc.iter().map(|i| &i.text).collect_vec();
        self.detail
            .content
            .jyc
            .iter()
            .filter(|t| !jyc.contains(t))
            .collect_vec()
    }

    pub fn other_fyc(&self) -> Vec<&String> {
        let fyc = self.fyc.iter().map(|i| &i.text).collect_vec();
        self.detail
            .content
            .fyc
            .iter()
            .filter(|t| !fyc.contains(t))
            .collect_vec()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperQuestionRef {}
