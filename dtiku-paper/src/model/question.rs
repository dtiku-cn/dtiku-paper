pub use super::_entities::question::*;
use super::{paper, Paper, PaperQuestion};
use crate::domain::question::QuestionSearch;
use anyhow::Context;
use itertools::Itertools;
use sea_orm::{
    sea_query::OnConflict, ColumnTrait, ConnectionTrait, DbErr, DerivePartialModel, EntityTrait,
    FromJsonQueryResult, FromQueryResult, QueryFilter,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum::Display;

pub struct Question {
    pub id: i32,
    pub content: String,
    pub extra: QuestionExtra,
    pub paper_id: i32,
    pub num: i16,
}

pub struct QuestionWithPaper {
    pub id: i32,
    pub content: String,
    pub extra: QuestionExtra,
    pub papers: Vec<PaperWithNum>,
}

pub struct PaperWithNum {
    pub paper: paper::Model,
    pub num: i16,
}

impl PaperWithNum {
    fn new(p: &paper::Model, sort: i16) -> Self {
        Self {
            paper: p.clone(),
            num: sort,
        }
    }
}

#[derive(Clone, Debug, DerivePartialModel, FromQueryResult)]
#[sea_orm(entity = "Entity")]
pub struct QuestionSelect {
    #[sea_orm(from_col = "id")]
    pub id: i32,
    #[sea_orm(from_col = "content")]
    pub content: String,
    #[sea_orm(from_col = "extra")]
    pub extra: QuestionExtra,
}

impl QuestionSelect {
    fn with_pid_num(self, num_map: &HashMap<i32, (i32, i16)>) -> Question {
        Question {
            id: self.id,
            content: self.content,
            extra: self.extra,
            paper_id: num_map
                .get(&self.id)
                .cloned()
                .map(|m| m.0)
                .unwrap_or_default(),
            num: num_map
                .get(&self.id)
                .cloned()
                .map(|m| m.1)
                .unwrap_or_default(),
        }
    }

    fn with_paper(
        self,
        qid_map: &HashMap<i32, Vec<super::paper_question::Model>>,
        id_paper: &HashMap<i32, paper::Model>,
    ) -> QuestionWithPaper {
        let papers = qid_map
            .get(&self.id)
            .map(|pqs| {
                pqs.into_iter()
                    .map(|pq| {
                        id_paper
                            .get(&pq.paper_id)
                            .map(|p| PaperWithNum::new(p, pq.sort))
                    })
                    .collect()
            })
            .collect();
        QuestionWithPaper {
            id: self.id,
            content: self.content,
            extra: self.extra,
            papers,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult, Display)]
#[serde(tag = "type")]
pub enum QuestionExtra {
    // 单选题
    #[serde(rename = "sc")]
    #[strum(serialize = "sc")]
    SingleChoice { options: Vec<QuestionChoice> },
    // 多选题
    #[serde(rename = "mc")]
    #[strum(serialize = "mc")]
    MultiChoice { options: Vec<QuestionChoice> },
    // 不定项选择题
    #[serde(rename = "ic")]
    #[strum(serialize = "ic")]
    IndefiniteChoice { options: Vec<QuestionChoice> },
    // 完形填空选择题
    #[serde(rename = "bc")]
    #[strum(serialize = "bc")]
    BlankChoice { options: Vec<QuestionChoice> },
    // 填空题
    #[serde(rename = "fb")]
    #[strum(serialize = "fb")]
    FillBlank,
    // 是非判断题
    #[serde(rename = "tf")]
    #[strum(serialize = "tf")]
    TrueFalse,
    // 分步式解答题
    #[serde(rename = "sqa")]
    #[strum(serialize = "sqa")]
    StepByStepQA(QA),
    // 封闭式解答题
    #[serde(rename = "cqa")]
    #[strum(serialize = "cqa")]
    ClosedEndedQA(QA),
    // 开放式解答题
    #[serde(rename = "oqa")]
    #[strum(serialize = "oqa")]
    OpenEndedQA(QA),
    // 听力题
    #[serde(rename = "lq")]
    #[strum(serialize = "lq")]
    ListenQuestion(String),
    // 选词题
    #[serde(rename = "ws")]
    #[strum(serialize = "ws")]
    WordSelection { options: Vec<QuestionChoice> },
    // 复合型
    #[serde(rename = "c")]
    #[strum(serialize = "c")]
    Compose { options: Vec<QuestionChoice> },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct QA {
    pub title: String,
    pub word_count: Option<i16>,
    pub material_ids: Vec<i32>,
}

pub type QuestionChoice = String;

impl Entity {
    pub async fn search_question<C>(
        db: &C,
        search: &QuestionSearch,
    ) -> anyhow::Result<Vec<QuestionWithPaper>>
    where
        C: ConnectionTrait,
    {
        let qs = Entity::find()
            .filter(search.clone())
            .into_partial_model::<QuestionSelect>()
            .all(db)
            .await
            .context("question search failed")?;

        let qids = qs.iter().map(|q| q.id).collect_vec();
        let pqs = PaperQuestion::find_by_question_id_in(db, qids).await?;
        let pids = pqs.iter().map(|pq| pq.paper_id).collect_vec();
        let qid_map = pqs
            .into_iter()
            .map(|pq| (pq.question_id, pq))
            .into_group_map();
        let papers = Paper::find_by_ids(db, pids).await?;
        let id_paper: HashMap<i32, paper::Model> = papers.into_iter().map(|p| (p.id, p)).collect();

        Ok(qs
            .into_iter()
            .map(|q| q.with_paper(&qid_map, &id_paper))
            .collect())
    }

    pub async fn find_by_paper_id<C>(db: &C, paper_id: i32) -> anyhow::Result<Vec<Question>>
    where
        C: ConnectionTrait,
    {
        let pms = PaperQuestion::find_by_paper_id(db, paper_id).await?;

        let id_pid_sort: HashMap<i32, (i32, i16)> = pms
            .iter()
            .map(|pm| (pm.question_id, (pm.paper_id, pm.sort)))
            .collect();

        let qids = id_pid_sort.keys().cloned().collect_vec();

        let questions = Entity::find()
            .filter(Column::Id.is_in(qids))
            .into_partial_model::<QuestionSelect>()
            .all(db)
            .await?;

        Ok(questions
            .into_iter()
            .map(|m| m.with_pid_num(&id_pid_sort))
            .sorted_by_key(|m| m.num)
            .collect())
    }
}

impl ActiveModel {
    pub async fn insert_on_conflict<C>(self, db: &C) -> Result<Model, DbErr>
    where
        C: ConnectionTrait,
    {
        Entity::insert(self)
            .on_conflict(
                OnConflict::columns([Column::Id])
                    .update_columns([Column::Content, Column::Extra, Column::Embedding])
                    .to_owned(),
            )
            .exec_with_returning(db)
            .await
    }
}
