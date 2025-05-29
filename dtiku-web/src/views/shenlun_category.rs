use super::GlobalVariables;
use askama::Template;

#[derive(Template)]
#[template(path = "shenlun-category.html.min.jinja")]
pub struct ShenlunCategoryTemplate {
    pub global: GlobalVariables,
}
