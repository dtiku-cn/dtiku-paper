use super::filters;
use super::GlobalVariables;
use crate::views::PageExt;
use askama::Template;
use askama_web::WebTemplate;
use dtiku_paper::model::question::QuestionExtra;
use dtiku_paper::{
    domain::keypoint::KeyPointTree,
    model::{question::QuestionWithPaper, question_keypoint_stats},
};
use spring_sea_orm::pagination::Page;

pub struct ShenlunCategoryQuery {
    pub kp_pid: i32,
    pub kp_id: i32,
    pub year: Option<i16>,
}

impl ShenlunCategoryQuery {
    pub fn build_url(&self) -> String {
        let Self {
            kp_id,
            kp_pid,
            year,
        } = self;
        if let Some(year) = year {
            format!("/shenlun-categories/{kp_pid}/{kp_id}/{year}")
        } else {
            format!("/shenlun-categories/{kp_pid}/{kp_id}")
        }
    }
}

#[derive(Template, WebTemplate)]
#[template(path = "shenlun-category.html.min.jinja")]
pub struct ShenlunCategoryTemplate {
    pub global: GlobalVariables,
    pub kp_tree: KeyPointTree,
    pub query: ShenlunCategoryQuery,
    pub years: Vec<question_keypoint_stats::Model>,
    pub page: Page<i32>,
    pub questions: Vec<QuestionWithPaper>,
}
