use super::PaperQuestion;
pub use super::_entities::question::*;
use itertools::Itertools;
use sea_orm::{
    sea_query::OnConflict, ColumnTrait, ConnectionTrait, DbErr, DerivePartialModel, EntityTrait,
    FromJsonQueryResult, FromQueryResult, QueryFilter,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

pub struct Question {
    pub id: i32,
    pub content: String,
    pub extra: QuestionExtra,
    pub num: i16,
}

#[derive(DerivePartialModel, FromQueryResult)]
#[sea_orm(entity = "Entity")]
struct QuestionSelect {
    #[sea_orm(from_col = "id")]
    id: i32,
    #[sea_orm(from_col = "content")]
    content: String,
    #[sea_orm(from_col = "extra")]
    extra: QuestionExtra,
}

impl QuestionSelect {
    fn with_num(self, num_map: &HashMap<i32, i16>) -> Question {
        Question {
            id: self.id,
            content: self.content,
            extra: self.extra,
            num: num_map.get(&self.id).cloned().unwrap_or_default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(tag = "type")]
pub enum QuestionExtra {
    // 单选题
    #[serde(rename = "sc")]
    SingleChoice { options: Vec<QuestionChoice> },
    // 多选题
    #[serde(rename = "mc")]
    MultiChoice { options: Vec<QuestionChoice> },
    // 不定项选择题
    #[serde(rename = "ic")]
    IndefiniteChoice { options: Vec<QuestionChoice> },
    // 完形填空选择题
    #[serde(rename = "bc")]
    BlankChoice { options: Vec<QuestionChoice> },
    // 填空题
    #[serde(rename = "fb")]
    FillBlank,
    // 是非判断题
    #[serde(rename = "tf")]
    TrueFalse,
    // 分步式解答题
    #[serde(rename = "sqa")]
    StepByStepQA(QA),
    // 封闭式解答题
    #[serde(rename = "cqa")]
    ClosedEndedQA(QA),
    // 开放式解答题
    #[serde(rename = "oqa")]
    OpenEndedQA(QA),
    // 听力题
    #[serde(rename = "lq")]
    ListenQuestion(String),
    // 选词题
    #[serde(rename = "ws")]
    WordSelection { options: Vec<QuestionChoice> },
    // 复合型
    #[serde(rename = "c")]
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
    pub async fn find_by_paper_id<C>(db: &C, paper_id: i32) -> anyhow::Result<Vec<Question>>
    where
        C: ConnectionTrait,
    {
        let pms = PaperQuestion::find_by_paper_id(db, paper_id).await?;

        let id_sort: HashMap<i32, i16> = pms.iter().map(|pm| (pm.question_id, pm.sort)).collect();

        let qids = id_sort.keys().cloned().collect_vec();

        let questions = Entity::find()
            .filter(Column::Id.is_in(qids))
            .into_partial_model::<QuestionSelect>()
            .all(db)
            .await?;

        Ok(questions
            .into_iter()
            .map(|m| m.with_num(&id_sort))
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
