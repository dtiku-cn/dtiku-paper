pub use super::_entities::question::*;
use super::{PaperQuestion, _entities::paper_question};
use itertools::Itertools;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DerivePartialModel, EntityTrait, FromQueryResult, QueryFilter,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

pub struct Question {
    pub id: i32,
    pub content: String,
    pub extra: QuestionExtra,
}

#[derive(DerivePartialModel, FromQueryResult)]
#[sea_orm(entity = "Entity")]
struct QuestionSelect {
    #[sea_orm(from_col = "id")]
    id: i32,
    #[sea_orm(from_col = "content")]
    content: String,
    #[sea_orm(from_col = "extra")]
    extra: Value,
}

impl TryFrom<QuestionSelect> for Question {
    type Error = anyhow::Error;

    fn try_from(value: QuestionSelect) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.id,
            content: value.content,
            extra: serde_json::from_value(value.extra)?,
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum QuestionExtra {
    // 单选题
    #[serde(rename = "sc")]
    SingleChoice(Vec<QuestionChoice>),
    // 多选题
    #[serde(rename = "mc")]
    MultiChoice(Vec<QuestionChoice>),
    // 不定项选择题
    #[serde(rename = "ic")]
    IndefiniteChoice(Vec<QuestionChoice>),
    // 完形填空选择题
    #[serde(rename = "bc")]
    BlankChoice(Vec<QuestionChoice>),
    // 是非判断题
    #[serde(rename = "tf")]
    TrueFalse,
    // 分步式解答题
    #[serde(rename = "sa")]
    StepByStepAnswer,
    // 封闭式解答题
    #[serde(rename = "ca")]
    ClosedEndedAnswer,
    // 开放式解答题
    #[serde(rename = "oa")]
    OpenEndedAnswer,
}

pub type QuestionChoice = String;

impl Entity {
    pub async fn find_by_paper_id<C>(db: &C, paper_id: i32) -> anyhow::Result<Vec<Question>>
    where
        C: ConnectionTrait,
    {
        let pms = PaperQuestion::find()
            .filter(paper_question::Column::PaperId.eq(paper_id))
            .all(db)
            .await?;

        let id_sort: HashMap<i32, i16> = pms.iter().map(|pm| (pm.question_id, pm.sort)).collect();

        let qids = id_sort.keys().cloned().collect_vec();

        let questions = Entity::find()
            .filter(Column::Id.is_in(qids))
            .into_partial_model::<QuestionSelect>()
            .all(db)
            .await?;

        questions
            .into_iter()
            .sorted_by_key(|m| id_sort.get(&m.id))
            .map(|m| m.try_into())
            .collect()
    }
}
