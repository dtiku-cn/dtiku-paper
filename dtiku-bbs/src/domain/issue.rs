use crate::model::TopicType;
use sea_orm::prelude::DateTime;

pub struct FullIssue {
    pub id: i32,
    pub topic: TopicType,
    pub title: String,
    pub markdown: String,
    pub user_id: i64,
    pub created: DateTime,
    pub modified: DateTime,
    pub view: i32,
    pub comment: i32,
}
