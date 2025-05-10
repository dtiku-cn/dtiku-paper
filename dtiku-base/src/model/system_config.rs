pub use super::_entities::system_config::*;
use super::enums::SystemConfigKey;
use anyhow::Context;
use dtiku_macros::cached;
use sea_orm::{
    sqlx::types::chrono::Local, ActiveModelBehavior, ColumnTrait, ConnectionTrait, DbErr,
    EntityTrait, QueryFilter, Set,
};
use serde::de::DeserializeOwned;
use spring::{async_trait, plugin::ComponentRegistry, tracing, App};
use spring_redis::{
    redis::{AsyncCommands, RedisError},
    Redis,
};

#[async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(mut self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if insert {
            self.created = Set(Local::now().naive_local());
        } else {
            let config_key = self.key.as_ref();
            let mut redis = App::global()
                .get_component::<Redis>()
                .expect("redis component don't exists");
            let _: () = redis.del(format!("config:{config_key:?}")).await.unwrap();
        }
        self.modified = Set(Local::now().naive_local());
        Ok(self)
    }
}

impl Entity {
    pub async fn find_all<C>(db: &C) -> anyhow::Result<Vec<Model>>
    where
        C: ConnectionTrait,
    {
        let all = Entity::find()
            .all(db)
            .await
            .context("find all config failed")?;

        Ok(all)
    }

    #[cached(key = "config:{key:?}")]
    pub async fn find_by_key<C>(db: &C, key: SystemConfigKey) -> anyhow::Result<Option<Model>>
    where
        C: ConnectionTrait,
    {
        Entity::find()
            .filter(Column::Key.eq(key))
            .one(db)
            .await
            .with_context(|| format!("system_config::find_by_key({key:?}) failed"))
    }
}
