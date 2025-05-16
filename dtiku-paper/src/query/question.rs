use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperQuestionQuery {
    #[serde(default)]
    pub paper_type: i32,
    #[serde(default)]
    pub paper_ids: Vec<i32>,
    #[serde(default)]
    pub keypoint_path: String,
    #[serde(default)]
    pub correct_ratio_start: f32,
    #[serde(default)]
    pub correct_ratio_end: f32,
}
