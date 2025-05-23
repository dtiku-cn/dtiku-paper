use super::GlobalVariables;
use askama::Template;
use chrono::NaiveDateTime;
use dtiku_bbs::model::{issue, IssueQuery, TopicType};
use spring_sea_orm::pagination::Page;
use strum::IntoEnumIterator;

#[derive(Template)]
#[template(path = "issue/list.html.min.jinja")]
pub struct ListIssueTemplate {
    pub global: GlobalVariables,
    pub page: Page<FullIssue>,
    pub query: IssueQuery,
}

pub struct FullIssue {
    pub id: i32,
    pub topic: TopicType,
    pub title: String,
    pub markdown: String,
    pub user_id: i64,
    pub created: NaiveDateTime,
    pub modified: NaiveDateTime,
    pub view: i32,
    pub comment: i32,
}

impl FullIssue {
    pub fn new(
        issue: issue::Model,
        page_pv: &std::collections::HashMap<String, i32>,
        page_comment: &std::collections::HashMap<String, i32>,
    ) -> Self {
        let key = format!("/bbs/issue/{}", issue.id);
        FullIssue {
            id: issue.id,
            title: issue.title,
            topic: issue.topic,
            markdown: issue.markdown,
            user_id: issue.user_id,
            created: issue.created,
            modified: issue.modified,
            view: page_pv.get(&key).unwrap_or(&0).to_owned(),
            comment: page_comment.get(&key).unwrap_or(&0).to_owned(),
        }
    }
}

#[derive(Template)]
#[template(path = "issue/issue.html.min.jinja")]
pub struct IssueTemplate {
    pub global: GlobalVariables,
    pub issue: FullIssue,
}
