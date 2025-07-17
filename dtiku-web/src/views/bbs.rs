use super::GlobalVariables;
use super::PageExt;
use crate::plugins::grpc_client::artalk::VoteStats;
use askama::Template;
use askama_web::WebTemplate;
use chrono::NaiveDateTime;
use dtiku_base::model::user_info;
use dtiku_bbs::model::{issue, IssueQuery, TopicType};
use spring_sea_orm::pagination::Page;
use strum::IntoEnumIterator;

#[derive(Template, WebTemplate)]
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
    pub vote_up: i64,
    pub vote_down: i64,
    pub user: Option<user_info::Model>,
}

impl FullIssue {
    pub fn new(
        issue: issue::Model,
        page_pv: &std::collections::HashMap<String, i32>,
        page_comment: &std::collections::HashMap<String, i32>,
        votes: &std::collections::HashMap<String, VoteStats>,
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
            vote_up: votes.get(&key).map(|v| v.vote_up).unwrap_or_default(),
            vote_down: votes.get(&key).map(|v| v.vote_down).unwrap_or_default(),
        }
    }

    pub fn author_name(&self) -> String {
        self.user
            .as_ref()
            .map_or_else(|| "未知用户".to_string(), |u| u.name.clone())
    }
}

#[derive(Template, WebTemplate)]
#[template(path = "issue/issue.html.min.jinja")]
pub struct IssueTemplate {
    pub global: GlobalVariables,
    pub issue: FullIssue,
}

#[derive(Template, WebTemplate)]
#[template(path = "issue/issue-editor.html.min.jinja")]
pub struct IssueEditorTemplate {
    pub global: GlobalVariables,
    pub issue: Option<FullIssue>,
}

trait TopicSelected {
    fn is_topic(&self, topic: &TopicType) -> bool;
}

impl TopicSelected for Option<FullIssue> {
    fn is_topic(&self, topic: &TopicType) -> bool {
        let t = self.as_ref().map(|i| i.topic);
        t == Some(topic.clone())
    }
}
