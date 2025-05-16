use super::GlobalVariables;
use askama::Template;
use dtiku_paper::{
    domain::question::QuestionSearch,
    model::{
        self,
        question::{QuestionExtra, QuestionWithPaper},
    },
};

#[derive(Template)]
#[template(path = "question-search.html.jinja")]
pub struct QuestionSearchTemplate {
    pub global: GlobalVariables,
    pub questions: Vec<QuestionWithPaper>,
    pub query: QuestionSearch,
}

#[derive(Template)]
#[template(path = "question-section.html.jinja")]
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

    pub(crate) fn option_len(&self) -> usize {
        self.extra.option_len()
    }
}
