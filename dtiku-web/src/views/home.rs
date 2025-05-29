use super::GlobalVariables;
use askama::Template;

#[derive(Template)]
#[template(path = "home.html.min.jinja")]
pub struct HomeTemplate {
    pub global: GlobalVariables,
}
