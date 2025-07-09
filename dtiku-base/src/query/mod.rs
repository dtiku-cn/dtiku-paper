use crate::model::user_info;
use chrono::Local;
use sea_orm::{sea_query::IntoCondition, ColumnTrait};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct UserQuery {
    name: Option<String>,
    gender: Option<bool>,
    expired: Option<bool>,
}

impl IntoCondition for UserQuery {
    fn into_condition(self) -> sea_orm::Condition {
        let mut condition = sea_orm::Condition::all();

        if let Some(name) = self.name {
            condition = condition.add(user_info::Column::Name.contains(name));
        }
        if let Some(gender) = self.gender {
            condition = condition.add(user_info::Column::Gender.eq(gender));
        }
        if let Some(expired) = self.expired {
            condition = condition.add(user_info::Column::Expired.lt(Local::now().naive_local()));
        }
        condition
    }
}
