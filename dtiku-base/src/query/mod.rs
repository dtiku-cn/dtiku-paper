use crate::model::user_info;
use chrono::Local;
use sea_orm::{sea_query::IntoCondition, ColumnTrait};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct UserQuery {
    id: Option<i32>,
    name: Option<String>,
    expired: Option<bool>,
}

impl IntoCondition for UserQuery {
    fn into_condition(self) -> sea_orm::Condition {
        let mut condition = sea_orm::Condition::all();

        if let Some(id) = self.id {
            condition = condition.add(user_info::Column::Id.eq(id));
        }
        if let Some(name) = self.name {
            condition = condition.add(user_info::Column::Name.contains(name));
        }
        if let Some(expired) = self.expired {
            condition = if expired {
                condition.add(user_info::Column::Expired.lte(Local::now().naive_local()))
            } else {
                condition.add(user_info::Column::Expired.gt(Local::now().naive_local()))
            }
        }
        condition
    }
}
