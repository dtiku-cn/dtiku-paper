use askama::Template;

use super::GlobalVariables;

#[derive(Template)]
#[template(path = "issue/list.html.jinja")]
pub struct ListIssueTemplate {
    pub global: GlobalVariables,
}
