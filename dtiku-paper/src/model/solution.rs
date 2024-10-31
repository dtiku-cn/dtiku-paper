pub use super::_entities::solution::*;
use sea_orm::{ActiveModelBehavior, ConnectionTrait};
use serde::{Deserialize, Serialize};

impl ActiveModelBehavior for ActiveModel {}

pub struct Solution {
    pub id: i32,
    pub question_id: i32,
    pub extra: SolutionExtra,
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
    pub async fn find_by_question_id<C>(db: &C, paper_id: i32) -> anyhow::Result<Vec<Solution>>
    where
        C: ConnectionTrait,
    {
        Ok(vec![])
    }
}
