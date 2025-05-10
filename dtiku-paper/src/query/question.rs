use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperQuestionQuery {
    pub paper_type: i32,
    pub paper_ids: Vec<i32>,
    pub keypoint_path: String,
    pub correct_ratio_start: f32,
    pub correct_ratio_end: f32,
}
