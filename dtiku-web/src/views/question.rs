use super::GlobalVariables;
use askama::Template;
use dtiku_paper::{
    domain::{keypoint::KeyPointPath, label::LabelTree, question::QuestionSearch},
    model::{
        self,
        question::{QuestionExtra, QuestionSinglePaper, QuestionWithPaper},
    },
    query::question::PaperQuestionQuery,
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
    pub questions: Vec<QuestionSinglePaper>,
    pub label_tree: LabelTree,
    pub query: PaperQuestionQuery,
    pub kp_paths: Vec<KeyPointPath>,
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
