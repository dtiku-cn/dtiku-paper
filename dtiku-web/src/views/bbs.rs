use super::GlobalVariables;
use askama::Template;
use dtiku_bbs::model::{issue, IssueQuery, TopicType};
use spring_sea_orm::pagination::Page;
use strum::IntoEnumIterator;

#[derive(Template)]
#[template(path = "issue/list.html.jinja")]
pub struct ListIssueTemplate {
    pub global: GlobalVariables,
    pub page: Page<issue::Model>,
    pub query: IssueQuery,
}
