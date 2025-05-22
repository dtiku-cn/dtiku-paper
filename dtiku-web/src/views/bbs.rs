use super::GlobalVariables;
use askama::Template;
use dtiku_bbs::{
    domain::issue::FullIssue,
    model::{IssueQuery, TopicType},
};
use spring_sea_orm::pagination::Page;
use strum::IntoEnumIterator;

#[derive(Template)]
#[template(path = "issue/list.html.min.jinja")]
pub struct ListIssueTemplate {
    pub global: GlobalVariables,
    pub page: Page<FullIssue>,
    pub query: IssueQuery,
}
