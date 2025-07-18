use super::GlobalVariables;
use super::PageExt;
use crate::query::idiom::IdiomReq;
use askama::Template;
use askama_web::WebTemplate;
use dtiku_paper::domain::label::LabelTree;
use dtiku_paper::model::question::QuestionExtra;
use dtiku_stats::{
    domain::{IdiomDetail, IdiomStats},
    model::sea_orm_active_enums::IdiomType,
};
use spring_sea_orm::pagination::Page;
use strum::IntoEnumIterator;

#[derive(Template, WebTemplate)]
#[template(path = "list-idiom.html.min.jinja")]
pub struct ListIdiomTemplate {
    pub global: GlobalVariables,
    pub model: IdiomType,
    pub label_tree: LabelTree,
    pub req: IdiomReq,
    pub page: Page<IdiomStats>,
}

#[derive(Template, WebTemplate)]
#[template(path = "idiom.html.min.jinja")]
pub struct IdiomDetailTemplate {
    pub global: GlobalVariables,
    pub idiom: IdiomDetail,
}
