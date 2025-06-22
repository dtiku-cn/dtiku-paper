use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPointNode {
    pub id: i32,
    pub name: String,
    pub pid: i32,
    pub exam_id: i16,
    pub paper_type: i16,
    pub children: Vec<KeyPointNode>,
}
