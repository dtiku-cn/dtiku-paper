use super::GlobalVariables;
use crate::views::paper::PaperType;
use askama::Template;
use askama_web::WebTemplate;
use dtiku_paper::model::paper;
use dtiku_stats::domain::IdiomStats;

#[derive(Template, WebTemplate)]
#[template(path = "home.html.min.jinja")]
pub struct HomeTemplate {
    pub global: GlobalVariables,
    pub home_papers: Vec<HomePapers>,
    pub idioms: Vec<IdiomStats>,
    pub words: Vec<IdiomStats>,
}

pub struct HomePapers {
    pub ty: PaperType,
    pub papers: Vec<paper::Model>,
    pub sub_papers: Vec<HomePapers>,
}
