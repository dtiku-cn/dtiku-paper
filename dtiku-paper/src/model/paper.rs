use serde::{Deserialize, Serialize};
use std::ops::Range;

pub struct Paper {
    id: i32,
    title: String,
    descrp: Option<String>,
    label_id: i32,
    extra: PaperExtra,
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
