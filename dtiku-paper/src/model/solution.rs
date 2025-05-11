pub use super::_entities::solution::*;
use anyhow::Context;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, FromJsonQueryResult, QueryFilter};
use serde::{Deserialize, Serialize};
use serde_with::{formats::CommaSeparator, serde_as, StringWithSeparator};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(tag = "type")]
pub enum SolutionExtra {
    // 单选题
    #[serde(rename = "sc")]
    SingleChoice(SingleChoice),
    // 多选题
    #[serde(rename = "mc")]
    MultiChoice(MultiChoice),
    // 不定项选择题
    #[serde(rename = "ic")]
    IndefiniteChoice(MultiChoice),
    // 完形填空选择题
    #[serde(rename = "bc")]
    BlankChoice(SingleChoice),
    // 填空题
    #[serde(rename = "fb")]
    FillBlank(FillBlank),
    // 是非判断题
    #[serde(rename = "tf")]
    TrueFalse(TrueFalseChoice),
    // 封闭式解答题
    #[serde(rename = "ce")]
    ClosedEndedQA(AnswerAnalysis),
    // 开放式解答题
    #[serde(rename = "oe")]
    OpenEndedQA(StepByStepAnswer),
    // 其他问答题
    #[serde(rename = "o")]
    OtherQA(OtherAnswer),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct SingleChoice {
    pub answer: u16,
    pub analysis: String,
}

#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct MultiChoice {
    #[serde_as(as = "StringWithSeparator::<CommaSeparator, u16>")]
    pub answer: Vec<u16>,
    pub analysis: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct TrueFalseChoice {
    pub answer: bool,
    pub analysis: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct FillBlank {
    pub blanks: Vec<String>,
    pub analysis: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct StepByStepAnswer {
    pub solution: Option<String>,
    pub analysis: Vec<StepAnalysis>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct OtherAnswer {
    pub answer: Option<String>,
    pub solution: Option<String>,
    pub analysis: Vec<StepAnalysis>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct StepAnalysis {
    pub label: String,
    pub content: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct AnswerAnalysis {
    pub answer: String,
    pub analysis: String,
}

impl Entity {
    pub async fn find_by_question_ids<C, IDS>(
        db: &C,
        question_ids: IDS,
    ) -> anyhow::Result<Vec<Model>>
    where
        C: ConnectionTrait,
        IDS: IntoIterator<Item = i32>,
    {
        Entity::find()
            .filter(Column::QuestionId.is_in(question_ids))
            .all(db)
            .await
            .with_context(|| format!("find_by_question_ids failed"))
    }
}
