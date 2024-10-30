use serde::{Deserialize, Serialize};
use std::ops::Range;

pub struct Label {
    id: i64,
    name: String,
    pid: i64,
}

pub struct Paper {
    id: i64,
    title: String,
    descrp: Option<String>,
    label_id: i64,
    extra: PaperExtra,
}

pub struct Question {
    id: i64,
    content: String,
    extra: QuestionExtra,
}

pub struct PaperQuestion {
    paper_id: i64,
    question_id: i64,
    sort: i32,
}

pub struct Material {
    id: i64,
    content: String,
}

pub struct PaperMaterial {
    paper_id: i64,
    material_id: i64,
    sort: i32,
}

pub struct QuestionMaterial{
    question_id: i64,
    material_id: i64,
}

pub struct Solution {
    id: i64,
    question_id: i64,
    extra: SolutionExtra,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PaperExtra {
    Chapter(Vec<PaperChapter>),
}

#[derive(Serialize, Deserialize)]
pub struct PaperChapter {
    name: String,
    desc: String,
    range: Range<i32>,
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
