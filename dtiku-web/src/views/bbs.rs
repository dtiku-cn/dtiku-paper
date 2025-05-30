use super::GlobalVariables;
use askama::Template;
use chrono::NaiveDateTime;
use dtiku_base::model::user_info;
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
    pub html: String,
    pub user_id: i32,
    pub created: NaiveDateTime,
    pub modified: NaiveDateTime,
    pub view: i32,
    pub comment: i32,
    pub user: Option<user_info::Model>,
}

impl FullIssue {
    pub fn new(
        issue: issue::Model,
        page_pv: &std::collections::HashMap<String, i32>,
        page_comment: &std::collections::HashMap<String, i32>,
        id_user_map: &mut std::collections::HashMap<i32, user_info::Model>,
    ) -> Self {
        let key = format!("/bbs/issue/{}", issue.id);
        FullIssue {
            user: id_user_map.remove(&issue.user_id),
            id: issue.id,
            title: issue.title,
            topic: issue.topic,
            markdown: issue.markdown,
            html: issue.html,
            user_id: issue.user_id,
            created: issue.created,
            modified: issue.modified,
            view: page_pv.get(&key).unwrap_or(&0).to_owned(),
            comment: page_comment.get(&key).unwrap_or(&0).to_owned(),
        }
    }

    pub fn author_name(&self) -> String {
        self.user
            .as_ref()
            .map_or_else(|| "未知用户".to_string(), |u| u.name.clone())
    }
}

#[derive(Template)]
#[template(path = "issue/issue.html.min.jinja")]
pub struct IssueTemplate {
    pub global: GlobalVariables,
    pub issue: FullIssue,
}

#[derive(Template)]
#[template(path = "issue/issue-editor.html.min.jinja")]
pub struct IssueEditorTemplate {
    pub global: GlobalVariables,
    pub issue: Option<FullIssue>,
}

impl IssueEditorTemplate {
    pub fn topic(&self) -> Option<TopicType> {
        self.issue.as_ref().map(|i| i.topic)
    }
}
