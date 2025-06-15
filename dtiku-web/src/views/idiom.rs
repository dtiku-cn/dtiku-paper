use crate::query::idiom::IdiomReq;
use super::GlobalVariables;
use askama::Template;
use dtiku_stats::StatsModelType;
use spring_sea_orm::pagination::Pagination;
use strum::IntoEnumIterator;

#[derive(Template)]
#[template(path = "list-idiom.html.min.jinja")]
pub struct ListIdiomTemplate {
    pub global: GlobalVariables,
    pub model: StatsModelType,
    pub req: IdiomReq,
    pub page: Pagination,
}
