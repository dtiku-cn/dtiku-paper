mod _entities;
pub mod issue;

pub use _entities::sea_orm_active_enums::TopicType;
use issue::Column;
pub use issue::Entity as Issue;
use sea_orm::{sea_query::IntoCondition, ColumnTrait, Condition};
use serde::{Deserialize, Serialize};
use strum::EnumMessage;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IssueQuery {
    pub title: Option<String>,
    pub topic: Option<TopicType>,
}

impl IntoCondition for IssueQuery {
    fn into_condition(self) -> sea_orm::Condition {
        let mut condition = Condition::all(); // 默认是 AND 条件组
        if let Some(topic) = self.topic {
            condition = condition.add(Column::Topic.eq(topic));
        }
        if let Some(title) = self.title {
            condition = condition.add(Column::Title.like(format!("%{}%", title)));
        }
        condition
    }
}

impl IssueQuery {
    pub fn topic(&self) -> &'static str {
        self.topic
            .as_ref()
            .and_then(|t| t.get_message())
            .unwrap_or_default()
    }

    pub fn title(&self) -> String {
        self.title.clone().unwrap_or_default()
    }

    pub fn to_qs(&self) -> String {
        serde_urlencoded::to_string(self).ok().unwrap_or_default()
    }
}
