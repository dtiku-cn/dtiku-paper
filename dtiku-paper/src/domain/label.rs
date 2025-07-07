use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::model::label;

#[derive(Debug, Serialize, Deserialize)]
pub struct LabelTree {
    pub labels: Vec<LabelNode>,
    pub level: bool,
}

impl LabelTree {
    pub fn none() -> Self {
        Self {
            labels: vec![],
            level: false,
        }
    }

    pub fn default_label_id(&self) -> i32 {
        if self.level {
            self.labels
                .iter()
                .find_map(|l| {
                    l.children
                        .as_ref()
                        .and_then(|children| children.first())
                        .map(|child| child.id)
                })
                .unwrap_or_default()
        } else {
            self.labels.first().map(|l| l.id).unwrap_or_default()
        }
    }

    pub fn label_text(&self, label_id: &i32) -> String {
        self.get_label(*label_id)
            .map(|l| l.name)
            .unwrap_or_default()
    }

    pub fn get_label(&self, label_id: i32) -> Option<LabelNode> {
        Self::inner_get_label(&self.labels, label_id)
    }

    fn inner_get_label(vec: &Vec<LabelNode>, id: i32) -> Option<LabelNode> {
        for l in vec {
            if l.id == id {
                return Some(l.clone());
            } else if let Some(children) = &l.children {
                let c = Self::inner_get_label(children, id);
                if c.is_some() {
                    return c;
                }
            }
        }
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelNode {
    pub id: i32,
    pub name: String,
    pub pid: i32,
    pub exam_id: i16,
    pub paper_type: i16,
    pub children: Option<Vec<LabelNode>>,
}

impl LabelNode {
    pub(crate) fn new(children: Option<Vec<label::Model>>, m: label::Model) -> Self {
        Self {
            id: m.id,
            name: m.name,
            pid: m.pid,
            exam_id: m.exam_id,
            paper_type: m.paper_type,
            children: children.map(|c| {
                c.into_iter()
                    .map(|m| LabelNode::new(None, m))
                    .sorted_by(|a, b| Ord::cmp(&a.name, &b.name))
                    .collect_vec()
            }),
        }
    }

    pub fn active(&self, label_id: &i32) -> bool {
        match &self.children {
            None => false,
            Some(children) => children.iter().any(|n| n.id == *label_id),
        }
    }
}
