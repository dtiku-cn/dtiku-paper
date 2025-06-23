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

    pub fn kp_text(&self, kp_id: &i32, default_value: &str) -> String {
        for node in &self.tree {
            if let Some(name) = Self::find_name_by_id(node, kp_id) {
                return name;
            }
        }
        default_value.to_string()
    }

    fn find_name_by_id(node: &KeyPointNode, kp_id: &i32) -> Option<String> {
        if &node.id == kp_id {
            return Some(node.name.clone());
        }
        for child in &node.children {
            if let Some(name) = Self::find_name_by_id(child, kp_id) {
                return Some(name);
            }
        }
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPointNode {
    pub id: i32,
    pub name: String,
    pub pid: i32,
    pub exam_id: i16,
    pub paper_type: i16,
    pub qcount: i64,
    pub children: Vec<KeyPointNode>,
}
