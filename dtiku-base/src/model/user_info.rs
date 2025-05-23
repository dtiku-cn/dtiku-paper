use anyhow::Context;
use sea_orm::{
    sqlx::types::chrono::Local, ActiveModelBehavior, ActiveValue::Set, ConnectionTrait, DbErr,
    EntityTrait,
};
use spring::async_trait;

pub use super::_entities::user_info::*;

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
    pub async fn find_user_by_id<C: ConnectionTrait>(
        db: &C,
        id: i32,
    ) -> anyhow::Result<Option<Model>> {
        Entity::find_by_id(id)
            .one(db)
            .await
            .with_context(|| format!("UserInfo::find_by_id({id}) failed"))
    }
}
