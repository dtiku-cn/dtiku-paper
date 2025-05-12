use askama::Template;

use super::GlobalVariables;

#[derive(Template)]
#[template(path = "home.html.jinja")]
pub struct HomeTemplate {
    pub global: GlobalVariables,
}
