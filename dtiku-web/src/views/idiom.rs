use askama::Template;

use super::GlobalVariables;

#[derive(Template)]
#[template(path = "list-idiom.html.jinja")]
pub struct ListIdiomTemplate {
    pub global: GlobalVariables,
}
