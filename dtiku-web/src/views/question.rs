use super::GlobalVariables;
use askama::Template;
use askama_web::WebTemplate;
use dtiku_paper::{
    domain::{keypoint::KeyPointPath, label::LabelTree, question::QuestionSearch},
    model::{
        self,
        question::{QuestionExtra, QuestionSinglePaper, QuestionWithPaper, QuestionWithSolutions},
    },
    query::question::{PaperQuestionQuery, SectionType},
};
use strum::IntoEnumIterator;

#[derive(Template, WebTemplate)]
#[template(path = "question/search.html.min.jinja")]
pub struct QuestionSearchTemplate {
    pub global: GlobalVariables,
    pub questions: Vec<QuestionWithPaper>,
    pub query: QuestionSearch,
}

#[derive(Template, WebTemplate)]
#[template(path = "question/search-img.html.min.jinja")]
pub struct QuestionSearchImgTemplate {
    pub global: GlobalVariables,
    pub questions: Vec<QuestionWithPaper>,
    pub query: QuestionSearch,
}

#[derive(Template, WebTemplate)]
#[template(path = "question/section.html.min.jinja")]
pub struct QuestionSectionTemplate {
    pub global: GlobalVariables,
    pub papers: Vec<model::paper::Model>,
    pub questions: Vec<QuestionSinglePaper>,
    pub label_tree: LabelTree,
    pub query: PaperQuestionQuery,
    pub kp_paths: Vec<KeyPointPath>,
}

#[derive(Template, WebTemplate)]
#[template(path = "question/detail.html.min.jinja")]
pub struct QuestionDetailTemplate {
    pub global: GlobalVariables,
    pub question: QuestionWithPaper,
}

#[derive(Template, WebTemplate)]
#[template(path = "question/only-comment.html.min.jinja")]
pub struct OnlyCommentTemplate {
    pub global: GlobalVariables,
}

#[derive(Template, WebTemplate)]
#[template(path = "question/recommend.html.min.jinja")]
pub struct QuestionRecommendTemplate {
    pub global: GlobalVariables,
    pub questions: Vec<QuestionWithPaper>,
}
