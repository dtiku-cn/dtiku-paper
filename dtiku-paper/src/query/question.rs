use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::OneOrMany;

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperQuestionQuery {
    #[serde(default)]
    pub paper_type: i32,
    #[serde_as(as = "OneOrMany<_>")]
    #[serde(default, rename = "paperIds")]
    pub paper_ids: Vec<i32>,
    #[serde(default, rename = "kp_path")]
    pub keypoint_path: String,
    #[serde(default, rename = "crs")]
    pub correct_ratio_start: f32,
    #[serde(default, rename = "cre")]
    pub correct_ratio_end: f32,
}
