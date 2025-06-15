use super::GlobalVariables;
use askama::Template;
use dtiku_stats::StatsModelType;
use strum::IntoEnumIterator;

#[derive(Template)]
#[template(path = "list-idiom.html.min.jinja")]
pub struct ListIdiomTemplate {
    pub global: GlobalVariables,
}
