use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PaperQuestionQuery {
    #[serde(default)]
    pub paper_type: i32,
    #[serde(default, rename = "paperIds")]
    pub paper_ids: Vec<i32>,
    #[serde(default, rename = "kp_path")]
    pub keypoint_path: String,
    #[serde(default, rename = "crs")]
    pub correct_ratio_start: f32,
    #[serde(default, rename = "cre")]
    pub correct_ratio_end: f32,
}
