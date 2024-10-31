pub use super::_entities::question::*;
use super::{PaperQuestion, _entities::paper_question};
use itertools::Itertools;
use sea_orm::{
    ActiveModelBehavior, ColumnTrait, ConnectionTrait, DerivePartialModel, EntityTrait,
    FromQueryResult, QueryFilter,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

impl ActiveModelBehavior for ActiveModel {}

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
    #[serde(rename = "s")]
    SingleOption(Vec<QuestionOption>),
    #[serde(rename = "m")]
    MultiOption(Vec<QuestionOption>),
}

pub type QuestionOption = String;

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
