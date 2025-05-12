use std::collections::HashMap;
use serde::Deserialize;
use strum::{AsRefStr, Display, EnumIter, EnumString};

#[derive(Default, Debug, Clone, Copy, Deserialize, Display, EnumIter, AsRefStr, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum PaperMode {
    Exercise,
    #[default]
    ShowAnswer,
}

pub struct FullPaper {
    pub mode: PaperMode,
    pub p: crate::model::paper::Model,
    pub qs: Vec<crate::model::question::Question>,
    pub ms: Vec<crate::model::material::Material>,
    pub ss: Vec<crate::model::solution::Model>,
    pub qid_mid_map: HashMap<i32, Vec<i32>>,
}

impl FullPaper {
    pub(crate) fn new(
        mode: PaperMode,
        p: crate::model::paper::Model,
        qs: Vec<crate::model::question::Question>,
        ms: Vec<crate::model::material::Material>,
        ss: Vec<crate::model::solution::Model>,
        qid_map: HashMap<i32, Vec<i32>>,
    ) -> Self {
        Self {
            mode,
            p,
            qs,
            ms,
            ss,
            qid_mid_map: qid_map,
        }
    }
}
