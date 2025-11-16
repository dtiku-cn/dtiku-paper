use super::filters;
use super::GlobalVariables;
use super::PageExt;
use crate::plugins::grpc_client::artalk::VoteStats;
use askama::Template;
use askama_web::WebTemplate;
use chrono::NaiveDateTime;
use dtiku_base::model::user_info;
use dtiku_bbs::model::issue::CollectIssueMarkdown;
use dtiku_bbs::model::issue::ListIssue;
use dtiku_bbs::model::{issue, IssueQuery, TopicType};
use spring_sea_orm::pagination::Page;
use strum::IntoEnumIterator;

#[derive(Template, WebTemplate)]
#[template(path = "issue/list.html.min.jinja")]
pub struct ListIssueTemplate {
    pub global: GlobalVariables,
    pub page: Page<FullIssue>,
    pub query: IssueQuery,
    pub pin_issues: Vec<ListIssue>,
}

pub struct FullIssue {
    pub id: i32,
    pub topic: TopicType,
    pub title: String,
    pub toc: String,
    pub markdown: String,
    pub html: String,
    pub user_id: i32,
    pub pin: bool,
    pub collect: bool,
    pub paid: bool,
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
        let (toc, markdown) = if issue.collect {
            let collect = serde_json::from_str::<CollectIssueMarkdown>(&issue.markdown)
                .expect("collect issue require CollectIssueMarkdown");
            (collect.toc, collect.content)
        } else {
            ("".to_string(), issue.markdown)
        };
        FullIssue {
            user: id_user_map.get(&issue.user_id).cloned(),
            id: issue.id,
            title: issue.title,
            topic: issue.topic,
            toc,
            markdown: markdown,
            html: issue.html,
            user_id: issue.user_id,
            pin: issue.pin,
            collect: issue.collect,
            paid: issue.paid,
            created: issue.created,
            modified: issue.modified,
            view: page_pv.get(&key).unwrap_or(&0).to_owned(),
            comment: page_comment.get(&key).unwrap_or(&0).to_owned(),
            vote_up: votes.get(&key).map(|v| v.vote_up).unwrap_or_default(),
            vote_down: votes.get(&key).map(|v| v.vote_down).unwrap_or_default(),
        }
    }

    pub fn new_for_list(
        issue: issue::ListIssue,
        page_pv: &std::collections::HashMap<String, i32>,
        page_comment: &std::collections::HashMap<String, i32>,
        votes: &std::collections::HashMap<String, VoteStats>,
        id_user_map: &mut std::collections::HashMap<i32, user_info::Model>,
    ) -> Self {
        let key = format!("/bbs/issue/{}", issue.id);
        FullIssue {
            user: id_user_map.get(&issue.user_id).cloned(),
            id: issue.id,
            title: issue.title,
            topic: issue.topic,
            toc: "".to_string(),
            markdown: "".to_string(),
            html: "".to_string(),
            user_id: issue.user_id,
            pin: issue.pin,
            collect: issue.collect,
            paid: issue.paid,
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
#[template(path = "issue/issue-content.html.min.jinja")]
pub struct IssueContentTemplate {
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
