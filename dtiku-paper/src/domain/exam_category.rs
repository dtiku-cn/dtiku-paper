use crate::model::{exam_category, FromType};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExamPaperType {
    pub id: i16,
    pub name: String,
    pub prefix: String,
    pub pid: i16,
    pub from_ty: FromType,
    pub children: Option<Vec<ExamPaperType>>,
}

impl ExamPaperType {
    pub(crate) fn new(
        children: Option<Vec<exam_category::Model>>,
        m: exam_category::Model,
    ) -> Self {
        Self {
            id: m.id,
            name: m.name,
            prefix: m.prefix,
            pid: m.pid,
            from_ty: m.from_ty,
            children: children.map(|c| {
                c.into_iter()
                    .map(|m| ExamPaperType::new(None, m))
                    .collect_vec()
            }),
        }
    }
}
