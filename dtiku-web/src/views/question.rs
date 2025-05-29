use super::GlobalVariables;
use askama::Template;
use chinese_number::{ChineseCase, ChineseCountMethod, ChineseVariant, NumberToChinese};
use dtiku_paper::{
    domain::question::QuestionSearch,
    model::{
        self,
        question::{QuestionExtra, QuestionWithPaper},
    },
};

#[derive(Template)]
#[template(path = "question-search.html.min.jinja")]
pub struct QuestionSearchTemplate {
    pub global: GlobalVariables,
    pub questions: Vec<QuestionWithPaper>,
    pub query: QuestionSearch,
}

#[derive(Template)]
#[template(path = "question-search-img.html.min.jinja")]
pub struct QuestionSearchImgTemplate {
    pub global: GlobalVariables,
    pub questions: Vec<QuestionWithPaper>,
    pub query: QuestionSearch,
}

#[derive(Template)]
#[template(path = "question-section.html.min.jinja")]
pub struct QuestionSectionTemplate {
    pub global: GlobalVariables,
    pub papers: Vec<model::paper::Model>,
    pub questions: Vec<model::question::Model>,
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

    pub(crate) fn chinese_num(&self) -> String {
        self.num
            .to_chinese(
                ChineseVariant::Traditional,
                ChineseCase::Lower,
                ChineseCountMethod::TenThousand,
            )
            .unwrap()
    }

    pub(crate) fn get_answer(&self) -> Option<String> {
        match &self.solutions {
            None => None,
            Some(ss) => ss.first().and_then(|s| s.extra.get_answer()),
        }
    }

    pub(crate) fn is_answer(&self, index0: &usize) -> bool {
        match &self.solutions {
            None => false,
            Some(ss) => ss
                .first()
                .map(|s| s.extra.is_answer(*index0))
                .unwrap_or_default(),
        }
    }
}
