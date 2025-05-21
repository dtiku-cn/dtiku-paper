use askama::Template;

use super::GlobalVariables;

#[derive(Template)]
#[template(path = "shenlun-category.html.min.jinja")]
pub struct ShenlunCategoryTemplate {
    pub global: GlobalVariables,
}
