use super::GlobalVariables;
use askama::Template;
use dtiku_paper::model::paper;

#[derive(Template)]
#[template(path = "home.html.min.jinja")]
pub struct HomeTemplate {
    pub global: GlobalVariables,
    pub xingce: Vec<paper::Model>,
    pub shenlun: Vec<paper::Model>,
}
