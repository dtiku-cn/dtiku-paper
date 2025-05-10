use askama::Template;

use super::GlobalVariables;

#[derive(Template)]
#[template(path = "home.html")]
pub struct HomeTemplate {
    pub global: GlobalVariables,
}
