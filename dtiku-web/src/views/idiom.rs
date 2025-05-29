use super::GlobalVariables;
use askama::Template;

#[derive(Template)]
#[template(path = "list-idiom.html.min.jinja")]
pub struct ListIdiomTemplate {
    pub global: GlobalVariables,
}
