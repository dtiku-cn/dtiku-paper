use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPointTree {
    pub tree: Vec<KeyPointNode>,
}

impl KeyPointTree {
    pub fn none() -> Self {
        Self { tree: vec![] }
    }

    pub fn default_kp(&self) -> (i32, i32) {
        match self.tree.first() {
            Some(root) => {
                let kp_id = root.children.first().map(|kp| kp.id).unwrap_or(0);
                (root.id, kp_id)
            }
            None => (0, 0),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPointNode {
    pub id: i32,
    pub name: String,
    pub pid: i32,
    pub exam_id: i16,
    pub paper_type: i16,
    pub children: Vec<KeyPointNode>,
}
