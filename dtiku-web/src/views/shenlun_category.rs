use super::GlobalVariables;
use askama::Template;
use dtiku_paper::model::question::QuestionExtra;
use dtiku_paper::{
    domain::keypoint::KeyPointTree,
    model::{question::QuestionWithPaper, question_keypoint_stats},
};

#[derive(Template)]
#[template(path = "shenlun-category.html.min.jinja")]
pub struct ShenlunCategoryTemplate {
    pub global: GlobalVariables,
    pub kp_tree: KeyPointTree,
    pub kp_pid: i32,
    pub kp_id: i32,
    pub year: Option<i16>,
    pub years: Vec<question_keypoint_stats::Model>,
    pub questions: Vec<QuestionWithPaper>,
}
