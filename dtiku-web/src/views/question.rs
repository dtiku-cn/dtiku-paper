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
#[template(path = "question/search.html.min.jinja")]
pub struct QuestionSearchTemplate {
    pub global: GlobalVariables,
    pub questions: Vec<QuestionWithPaper>,
    pub query: QuestionSearch,
}

#[derive(Template)]
#[template(path = "question/search-img.html.min.jinja")]
pub struct QuestionSearchImgTemplate {
    pub global: GlobalVariables,
    pub questions: Vec<QuestionWithPaper>,
    pub query: QuestionSearch,
}

#[derive(Template)]
#[template(path = "question/section.html.min.jinja")]
pub struct QuestionSectionTemplate {
    pub global: GlobalVariables,
    pub papers: Vec<model::paper::Model>,
    pub questions: Vec<QuestionWithPaper>,
}

#[derive(Template)]
#[template(path = "question/detail.html.min.jinja")]
pub struct QuestionDetailTemplate {
    pub global: GlobalVariables,
    pub question: QuestionWithPaper,
}

#[derive(Template)]
#[template(path = "question/only-comment.html.min.jinja")]
pub struct OnlyCommentTemplate {
    pub global: GlobalVariables,
}
