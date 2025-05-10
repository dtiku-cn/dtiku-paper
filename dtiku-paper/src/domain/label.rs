use serde::{Deserialize, Serialize};

use crate::model::label;

#[derive(Debug, Serialize, Deserialize)]
pub struct LabelTree {
    pub labels: Vec<LabelNode>,
    pub level: bool,
}

impl LabelTree {
    pub fn default_label_id(&self) -> i32 {
        if self.level {
            self.labels.first().map(|l| l.id).unwrap_or_default()
        } else {
            self.labels
                .iter()
                .find_map(|l| {
                    l.children
                        .as_ref()
                        .and_then(|children| children.first())
                        .map(|child| child.id)
                })
                .unwrap_or_default()
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LabelNode {
    pub id: i32,
    pub name: String,
    pub pid: i32,
    pub exam_id: i16,
    pub paper_type: i16,
    pub children: Option<Vec<label::Model>>,
}

impl LabelNode {
    pub(crate) fn new(children: Option<Vec<label::Model>>, m: label::Model) -> Self {
        Self {
            id: m.id,
            name: m.name,
            pid: m.pid,
            exam_id: m.exam_id,
            paper_type: m.paper_type,
            children,
        }
    }
}
