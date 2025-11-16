use dtiku_bbs::model::TopicType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueReq {
    pub topic: TopicType,
    pub title: String,
    pub markdown: String,
    pub html: String,
    #[serde(default)]
    pub paid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueDetailReq {
    #[serde(default)]
    pub html: bool,
}
