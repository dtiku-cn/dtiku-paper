use serde::{Deserialize, Serialize};
use std::ops::Range;

pub struct Label {
    id: i32,
    name: String,
    pid: i32,
}

pub struct Paper {
    id: i32,
    title: String,
    descrp: Option<String>,
    label_id: i32,
    extra: PaperExtra,
}

pub struct Question {
    id: i32,
    content: String,
    extra: QuestionExtra,
}

pub struct PaperQuestion {
    paper_id: i32,
    question_id: i32,
    sort: i16,
}

pub struct Material {
    id: i32,
    content: String,
}

pub struct PaperMaterial {
    paper_id: i32,
    material_id: i32,
    sort: i16,
}

pub struct QuestionMaterial {
    question_id: i32,
    material_id: i32,
}

pub struct Solution {
    id: i32,
    question_id: i32,
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
    range: Range<i16>,
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
