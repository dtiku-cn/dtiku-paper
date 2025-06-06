pub use super::_entities::user_info::*;
use anyhow::Context;
use spring_redis::cache;
use sea_orm::{
    sqlx::types::chrono::Local, ActiveModelBehavior, ActiveValue::Set, ColumnTrait,
    ConnectionTrait, DbErr, EntityTrait, QueryFilter,
};
use spring::async_trait;

impl Model {
    pub fn is_expired(&self) -> bool {
        true
    }

    pub fn due_time(&self) -> String {
        "2023-10-01".to_string()
    }
}

#[async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(mut self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if insert {
            self.created = Set(Local::now().naive_local());
        }
        self.modified = Set(Local::now().naive_local());
        Ok(self)
    }
}

impl Entity {
    #[cache("user:{id}", expire = 86400)]
    pub async fn find_user_by_id<C: ConnectionTrait>(
        db: &C,
        id: i32,
    ) -> anyhow::Result<Option<Model>> {
        Entity::find_by_id(id)
            .one(db)
            .await
            .with_context(|| format!("UserInfo::find_user_by_id({id}) failed"))
    }

    pub async fn find_user_by_ids<C: ConnectionTrait>(
        db: &C,
        ids: Vec<i32>,
    ) -> anyhow::Result<Vec<Model>> {
        Entity::find()
            .filter(Column::Id.is_in(ids))
            .all(db)
            .await
            .context("UserInfo::find_user_by_ids() failed")
    }
}
