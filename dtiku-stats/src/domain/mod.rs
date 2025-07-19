use crate::model::{
    idiom::{self, BriefIdiom},
    idiom_ref,
};
use dtiku_paper::model::{paper, question};
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

    pub(crate) fn from_brief(
        brief_idiom: Option<&BriefIdiom>,
        s: IdiomRefStatsWithoutLabel,
    ) -> Self {
        Self {
            text: brief_idiom.map(|i| i.text.clone()).unwrap_or_default(),
            explain: brief_idiom.map(|i| i.explain.clone()).unwrap_or_default(),
            idiom_id: s.idiom_id,
            question_count: s.question_count,
            paper_count: s.paper_count,
        }
    }
}

#[derive(Debug, Clone)]
pub struct IdiomDetail {
    pub detail: idiom::Model,
    pub refs: Vec<PaperQuestionRef>,
    pub jyc: Vec<IdiomStats>,
    pub fyc: Vec<IdiomStats>,
}

impl IdiomDetail {
    pub(crate) fn new(
        detail: idiom::Model,
        refs: Vec<PaperQuestionRef>,
        jyc: Vec<IdiomStats>,
        fyc: Vec<IdiomStats>,
    ) -> Self {
        Self {
            detail,
            refs,
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

#[derive(Debug, Clone)]
pub struct PaperQuestionRef {
    pub paper: paper::Model,
    pub question: question::QuestionWithSolutions,
    pub sort: i16,
    pub id: i32,
}

impl PaperQuestionRef {
    pub(crate) fn new(
        r: idiom_ref::Model,
        p: Option<&paper::Model>,
        q: Option<&question::QuestionWithSolutions>,
    ) -> Self {
        Self {
            paper: p.cloned().unwrap(),
            question: q.cloned().unwrap(),
            sort: r.sort,
            id: r.id,
        }
    }
}
