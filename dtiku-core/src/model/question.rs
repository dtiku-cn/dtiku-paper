use serde::{Deserialize, Serialize};

pub struct Question {
    id: i32,
    content: String,
    extra: QuestionExtra,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum QuestionExtra {
    #[serde(rename = "single")]
    SingleOption(Vec<QuestionOption>),
    #[serde(rename = "multi")]
    MultiOption(Vec<QuestionOption>),
}

pub type QuestionOption = String;
