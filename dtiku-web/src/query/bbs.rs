use dtiku_bbs::model::TopicType;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IssueReq {
    pub topic: TopicType,
    pub title: String,
    pub markdown: String,
    pub html: String,
}
