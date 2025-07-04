use super::GlobalVariables;
use crate::query::idiom::IdiomReq;
use askama::Template;
use dtiku_paper::domain::label::LabelTree;
use dtiku_stats::{domain::IdiomDetail, model::sea_orm_active_enums::IdiomType};
use spring_sea_orm::pagination::Page;
use strum::IntoEnumIterator;

#[derive(Template)]
#[template(path = "list-idiom.html.min.jinja")]
pub struct ListIdiomTemplate {
    pub global: GlobalVariables,
    pub model: IdiomType,
    pub label_tree: LabelTree,
    pub req: IdiomReq,
    pub page: Page<()>,
}

#[derive(Template)]
#[template(path = "idiom.html.min.jinja")]
pub struct IdiomDetailTemplate {
    pub global: GlobalVariables,
    pub idiom: IdiomDetail,
}
