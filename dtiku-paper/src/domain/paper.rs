pub struct FullPaper {
    p: crate::model::paper::Model,
    qs: Vec<crate::model::question::Question>,
    ms: Vec<crate::model::material::Material>,
    ss: Vec<crate::model::solution::Solution>,
}

impl FullPaper {
    pub(crate) fn new(
        p: crate::model::paper::Model,
        qs: Vec<crate::model::question::Question>,
        ms: Vec<crate::model::material::Material>,
        ss: Vec<crate::model::solution::Solution>,
    ) -> Self {
        Self { p, qs, ms, ss }
    }
}
