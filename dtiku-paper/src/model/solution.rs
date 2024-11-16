pub use super::_entities::solution::*;
use anyhow::Context;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

pub struct Solution {
    pub id: i32,
    pub question_id: i32,
    pub extra: SolutionExtra,
}

impl TryFrom<Model> for Solution {
    type Error = anyhow::Error;

    fn try_from(value: Model) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.id,
            question_id: value.question_id,
            extra: serde_json::from_value(value.extra)?,
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SolutionExtra {
    // 单选题
    #[serde(rename = "sc")]
    SingleChoice { answer: u32, analysis: String },
    // 多选题
    #[serde(rename = "mc")]
    MultiChoice { answers: Vec<u32>, analysis: String },
    // 不定项选择题
    #[serde(rename = "ic")]
    IndefiniteChoice { answer: Vec<u32>, analysis: String },
    // 完形填空选择题
    #[serde(rename = "bc")]
    BlankChoice { answer: u32, analysis: String },
    // 是非判断题
    #[serde(rename = "tf")]
    TrueFalse { answer: bool, analysis: String },
    // 分步式解答题
    #[serde(rename = "sa")]
    StepByStepAnswer { analysis: Vec<String> },
    // 封闭式解答题
    #[serde(rename = "ce")]
    ClosedEndedAnswer { answer: String, analysis: String },
    // 开放式解答题
    #[serde(rename = "oe")]
    OpenEndedAnswer { answer: String, analysis: String },
}

impl Entity {
    pub async fn find_by_question_ids<C, IDS>(
        db: &C,
        question_ids: IDS,
    ) -> anyhow::Result<Vec<Solution>>
    where
        C: ConnectionTrait,
        IDS: IntoIterator<Item = i32>,
    {
        Entity::find()
            .filter(Column::QuestionId.is_in(question_ids))
            .all(db)
            .await
            .with_context(|| format!("find_by_question_ids failed"))?
            .into_iter()
            .map(|m| m.try_into())
            .collect()
    }
}
