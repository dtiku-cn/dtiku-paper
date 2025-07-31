pub use super::_entities::question::*;
use super::{paper, Paper, PaperQuestion, _entities::solution, material};
use crate::{
    domain::question::QuestionSearch,
    model::{paper_question, Solution},
};
use anyhow::Context;
use itertools::Itertools;
use sea_orm::{
    prelude::PgVector, sea_query::OnConflict, ActiveValue::Set, ColumnTrait, ConnectionTrait,
    DerivePartialModel, EntityTrait, FromJsonQueryResult, FromQueryResult, QueryFilter,
    QuerySelect, Statement,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum::Display;

macro_rules! question_methods {
    () => {
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

        pub fn abbr(&self, size: usize) -> &str {
            let text = {
                let html = scraper::Html::parse_fragment(&self.content);
                html.root_element().text().join("")
            };
            text
                .char_indices()
                .nth(size)
                .map(|(idx, _)| &self.content[..idx])
                .unwrap_or(&self.content)
        }
    };
}

#[derive(Clone, Debug)]
pub struct Question {
    pub id: i32,
    pub content: String,
    pub extra: QuestionExtra,
    pub paper_id: i32,
    pub num: i16,
}

#[derive(Clone, Debug)]
pub struct QuestionWithSolutions {
    pub id: i32,
    pub content: String,
    pub extra: QuestionExtra,
    pub solutions: Option<Vec<solution::Model>>,
}

impl QuestionWithSolutions {
    question_methods!();
}

#[derive(Clone, Debug)]
pub struct QuestionSinglePaper {
    pub id: i32,
    pub content: String,
    pub extra: QuestionExtra,
    pub paper: PaperWithNum,
    pub solutions: Option<Vec<solution::Model>>,
    pub materials: Option<Vec<material::Model>>,
}

impl QuestionSinglePaper {
    pub(crate) fn new(
        q: Model,
        pid_map: &HashMap<i32, &paper::Model>,
        qid_map: &mut HashMap<i32, paper_question::Model>,
        id_material_map: &mut HashMap<i32, material::Model>,
        qm_map: &mut HashMap<i32, Vec<i32>>,
    ) -> Self {
        let pq = qid_map.remove(&q.id).unwrap();
        let p = pid_map.get(&pq.paper_id).unwrap();
        let mids = qm_map.remove(&q.id);
        let materials: Option<Vec<material::Model>> = match mids {
            None => None,
            Some(mids) => Some(
                mids.into_iter()
                    .filter_map(|mid| id_material_map.remove(&mid))
                    .collect(),
            ),
        };
        Self {
            id: q.id,
            content: q.content,
            extra: q.extra,
            paper: PaperWithNum::new(p, pq.sort),
            solutions: None,
            materials,
        }
    }
    question_methods!();
}

#[derive(Clone, Debug)]
pub struct QuestionWithPaper {
    pub id: i32,
    pub content: String,
    pub extra: QuestionExtra,
    pub papers: Vec<PaperWithNum>,
    pub solutions: Option<Vec<solution::Model>>,
    pub materials: Option<Vec<material::Model>>,
}

impl QuestionWithPaper {
    pub fn new(
        q: Model,
        papers: Vec<PaperWithNum>,
        solutions: Option<Vec<solution::Model>>,
        materials: Option<Vec<material::Model>>,
    ) -> Self {
        Self {
            id: q.id,
            content: q.content,
            extra: q.extra,
            papers,
            solutions,
            materials,
        }
    }
    question_methods!();
}

#[derive(Clone, Debug)]
pub struct PaperWithNum {
    pub paper: paper::Model,
    pub num: i16,
}

impl PaperWithNum {
    pub fn new(p: &paper::Model, sort: i16) -> Self {
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

    fn with_solutions(
        self,
        solution_map: &HashMap<i32, Vec<solution::Model>>,
    ) -> QuestionWithSolutions {
        QuestionWithSolutions {
            id: self.id,
            content: self.content,
            extra: self.extra,
            solutions: solution_map.get(&self.id).cloned(),
        }
    }

    fn with_paper(
        self,
        qid_map: &HashMap<i32, Vec<super::paper_question::Model>>,
        id_paper: &HashMap<i32, paper::Model>,
    ) -> QuestionWithPaper {
        let papers = qid_map.get(&self.id).map(|pqs| {
            pqs.into_iter()
                .filter_map(|pq| {
                    id_paper
                        .get(&pq.paper_id)
                        .map(|p| PaperWithNum::new(p, pq.sort))
                })
                .collect::<Vec<_>>()
        });
        QuestionWithPaper {
            id: self.id,
            content: self.content,
            extra: self.extra,
            solutions: None,
            papers: papers.unwrap_or_default(),
            materials: None,
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
    // 填空问答题
    #[serde(rename = "ba")]
    #[strum(serialize = "ba")]
    BlankAnswer,
    // 是非判断题
    #[serde(rename = "tf")]
    #[strum(serialize = "tf")]
    TrueFalse,
    // 分步式解答题
    #[serde(rename = "sqa")]
    #[strum(serialize = "sqa")]
    StepByStepQA { qa: Vec<QA> },
    // 封闭式解答题
    #[serde(rename = "cqa")]
    #[strum(serialize = "cqa")]
    ClosedEndedQA { qa: Vec<QA> },
    // 开放式解答题
    #[serde(rename = "oqa")]
    #[strum(serialize = "oqa")]
    OpenEndedQA { qa: Vec<QA> },
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

impl QuestionExtra {
    pub fn option_len(&self) -> usize {
        match &self {
            Self::SingleChoice { options }
            | Self::MultiChoice { options }
            | Self::IndefiniteChoice { options }
            | Self::BlankChoice { options } => options.iter().map(|o| o.len()).sum(),
            _ => 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct QA {
    pub title: String,
    pub word_count: Option<i16>,
    pub material_ids: Vec<i32>,
}

pub type QuestionChoice = String;

impl Entity {
    pub async fn find_by_ids<C>(db: &C, ids: Vec<i32>) -> anyhow::Result<Vec<Model>>
    where
        C: ConnectionTrait,
    {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        Entity::find()
            .filter(Column::Id.is_in(ids))
            .all(db)
            .await
            .context("question::find_by_ids() failed")
    }

    pub async fn find_by_ids_with_solutions<C>(
        db: &C,
        ids: Vec<i32>,
    ) -> anyhow::Result<Vec<QuestionWithSolutions>>
    where
        C: ConnectionTrait,
    {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let qs = Entity::find()
            .filter(Column::Id.is_in(ids.clone()))
            .into_partial_model::<QuestionSelect>()
            .all(db)
            .await
            .context("question::find_by_ids() failed")?;

        let ss = Solution::find_by_question_ids(db, ids).await?;
        let solution_map = ss.into_iter().into_group_map_by(|s| s.question_id);

        Ok(qs
            .into_iter()
            .map(|q| q.with_solutions(&solution_map))
            .collect())
    }

    pub async fn search_question<C>(
        db: &C,
        search: &QuestionSearch,
    ) -> anyhow::Result<Vec<QuestionWithPaper>>
    where
        C: ConnectionTrait,
    {
        let qs = Entity::find()
            .filter(search.clone())
            .limit(100)
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

    pub async fn find_by_embedding<C>(db: &C, embedding: Vec<f32>) -> anyhow::Result<Vec<Model>>
    where
        C: ConnectionTrait,
    {
        Model::find_by_statement(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            r#"
                SELECT *
                FROM question
                ORDER BY embedding <-> $1
                LIMIT 10
            "#,
            vec![PgVector::from(embedding).into()],
        ))
        .all(db)
        .await
        .context("Question::find_by_embedding() failed")
    }
}

impl ActiveModel {
    pub async fn insert_on_conflict<C>(mut self, db: &C) -> anyhow::Result<Model>
    where
        C: ConnectionTrait,
    {
        if let Some(embedding) = self.embedding.take() {
            let embedding_vec = embedding.to_vec();
            let content = self.content.take().unwrap();
            let qs = Entity::find_by_embedding(db, embedding_vec).await?;
            for q in qs {
                if q.content == content {
                    return Ok(q);
                }
                if q.content.len() > 100 && content.len() > 100 {
                    let edit_distance = textdistance::str::levenshtein(&q.content, &content);
                    // 95%相似度
                    if edit_distance * 20 < content.len().max(q.content.len()) {
                        return Ok(q);
                    }
                }
            }
            self.embedding = Set(embedding);
            self.content = Set(content);
        }
        Entity::insert(self)
            .on_conflict(
                OnConflict::columns([Column::Id])
                    .update_columns([Column::Content, Column::Extra, Column::Embedding])
                    .to_owned(),
            )
            .exec_with_returning(db)
            .await
            .context("insert question failed")
    }
}
