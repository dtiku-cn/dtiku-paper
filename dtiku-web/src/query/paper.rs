use dtiku_paper::domain::paper::PaperMode;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ListPaperQuery {
    #[serde(rename = "ty")]
    pub paper_type_prefix: String,
    #[serde(default, rename = "lid")]
    pub label_id: i32,
}

#[derive(Debug, Deserialize)]
pub struct PaperQuery {
    pub mode: Option<PaperMode>,
}

#[derive(Debug, Deserialize)]
pub struct PaperTitleLikeQuery {
    #[serde(default, rename = "q")]
    pub title: String,
}
