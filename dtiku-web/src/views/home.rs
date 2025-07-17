use super::GlobalVariables;
use askama::Template;
use askama_web::WebTemplate;
use dtiku_paper::model::paper;
use dtiku_stats::domain::IdiomStats;

#[derive(Template, WebTemplate)]
#[template(path = "home.html.min.jinja")]
pub struct HomeTemplate {
    pub global: GlobalVariables,
    pub xingce: Vec<paper::Model>,
    pub shenlun: Vec<paper::Model>,
    pub idioms: Vec<IdiomStats>,
    pub words: Vec<IdiomStats>,
}
