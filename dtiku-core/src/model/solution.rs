use serde::{Deserialize, Serialize};

pub struct Solution {
    id: i32,
    question_id: i32,
    extra: SolutionExtra,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SolutionExtra {
    #[serde(rename = "single")]
    SingleOption { answer: u32, analysis: String },
    #[serde(rename = "multi")]
    MultiOption { answers: Vec<u32>, analysis: String },
    #[serde(rename = "qa")]
    QA { answer: String, analysis: String },
}
