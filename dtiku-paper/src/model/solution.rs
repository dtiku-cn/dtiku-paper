pub use super::_entities::solution::*;
use anyhow::Context;
use sea_orm::{ActiveModelBehavior, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

impl ActiveModelBehavior for ActiveModel {}

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
    #[serde(rename = "s")]
    SingleOption { answer: u32, analysis: String },
    #[serde(rename = "m")]
    MultiOption { answers: Vec<u32>, analysis: String },
    #[serde(rename = "qa")]
    QA { answer: String, analysis: String },
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
