pub use super::_entities::solution::*;
use anyhow::Context;
use itertools::Itertools;
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
    // 填空题
    #[serde(rename = "ba")]
    BlankAnswer(BlankAnswer),
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

impl SolutionExtra {
    pub fn is_answer(&self, index0: usize) -> bool {
        let answer_index = index0 as u8;
        match self {
            Self::SingleChoice(SingleChoice { answer, .. })
            | Self::BlankChoice(SingleChoice { answer, .. }) => *answer == answer_index,
            Self::MultiChoice(MultiChoice { answer, .. })
            | Self::IndefiniteChoice(MultiChoice { answer, .. }) => answer.contains(&answer_index),
            Self::TrueFalse(TrueFalseChoice { answer, .. }) => *answer && answer_index == 0,
            _ => false,
        }
    }

    pub fn get_answer(&self) -> Option<String> {
        match self {
            Self::SingleChoice(SingleChoice { answer, .. })
            | Self::BlankChoice(SingleChoice { answer, .. }) => Some(Self::convert_answer(*answer)),
            Self::MultiChoice(MultiChoice { answer, .. })
            | Self::IndefiniteChoice(MultiChoice { answer, .. }) => {
                Some(answer.iter().map(|a| Self::convert_answer(*a)).join(", "))
            }
            Self::TrueFalse(TrueFalseChoice { answer, .. }) => {
                if *answer {
                    Some("T".to_string())
                } else {
                    Some("F".to_string())
                }
            }
            _ => None,
        }
    }

    fn convert_answer(answer: u8) -> String {
        let c = (b'A' + answer) as char;
        c.to_string()
    }

    pub fn get_html(&self) -> String {
        match self {
            Self::SingleChoice(SingleChoice { analysis, .. })
            | Self::BlankChoice(SingleChoice { analysis, .. })
            | Self::MultiChoice(MultiChoice { analysis, .. })
            | Self::IndefiniteChoice(MultiChoice { analysis, .. })
            | Self::TrueFalse(TrueFalseChoice { analysis, .. })
            | Self::FillBlank(FillBlank { analysis, .. })
            | Self::BlankAnswer(BlankAnswer { analysis, .. })
            | Self::ClosedEndedQA(AnswerAnalysis { analysis, .. }) => analysis.to_string(),
            Self::OpenEndedQA(StepByStepAnswer { analysis, .. })
            | Self::OtherQA(OtherAnswer { analysis, .. }) => {
                analysis.iter().map(|a| a.content.as_str()).join("。")
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct SingleChoice {
    pub answer: u8,
    pub analysis: String,
}

#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct MultiChoice {
    #[serde_as(as = "StringWithSeparator::<CommaSeparator, u8>")]
    pub answer: Vec<u8>,
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
pub struct BlankAnswer {
    pub answer: String,
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
    pub async fn find_by_qid<C>(db: &C, question_id: i32) -> anyhow::Result<Vec<Model>>
    where
        C: ConnectionTrait,
    {
        Entity::find()
            .filter(Column::QuestionId.eq(question_id))
            .all(db)
            .await
            .with_context(|| format!("find_by_question_id({question_id}) failed"))
    }

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
