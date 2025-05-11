use super::GlobalVariables;
use askama::Template;
use dtiku_paper::model::{self, question::QuestionExtra};

#[derive(Template)]
#[template(path = "question-search.html")]
pub struct QuestionSearchTemplate {
    pub global: GlobalVariables,
}

#[derive(Template)]
#[template(path = "question-section.html")]
pub struct QuestionSectionTemplate {
    pub global: GlobalVariables,
}

pub struct FullQuestion {
    pub id: i32,
    pub content: String,
    pub extra: QuestionExtra,
    pub num: i16,
    pub materials: Option<Vec<model::material::Material>>,
    pub solutions: Option<Vec<model::solution::Model>>,
    pub chapter: Option<model::paper::PaperChapter>,
}

impl FullQuestion {
    pub(crate) fn new(
        materials: Option<Vec<model::material::Material>>,
        solutions: Option<Vec<model::solution::Model>>,
        chapter: Option<model::paper::PaperChapter>,
        q: model::question::Question,
    ) -> Self {
        Self {
            id: q.id,
            content: q.content,
            extra: q.extra,
            num: q.num,
            materials,
            solutions,
            chapter,
        }
    }
}
