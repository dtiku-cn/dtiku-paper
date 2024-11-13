pub use super::_entities::system_config::*;
use super::enums::SystemConfigKey;
use anyhow::Context;
use sea_orm::{
    sqlx::types::chrono::Local, ActiveModelBehavior, ColumnTrait, ConnectionTrait, DbErr,
    EntityTrait, QueryFilter, Set,
};
use serde::de::DeserializeOwned;
use spring::{async_trait, App};
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
            let cache_key = self.key.as_ref().as_ref();
            let mut redis = App::global()
                .get_component::<Redis>()
                .expect("redis component don't exists");
            let _: () = redis.del(&format!("config:{cache_key}")).await.unwrap();
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

    pub async fn find_by_key<C>(db: &C, key: SystemConfigKey) -> anyhow::Result<Option<Model>>
    where
        C: ConnectionTrait,
    {
        Ok(Entity::find().filter(Column::Key.eq(key)).one(db).await?)
    }

    pub async fn find_value_by_key<C, T>(db: &C, key: SystemConfigKey) -> anyhow::Result<Option<T>>
    where
        C: ConnectionTrait,
        T: DeserializeOwned,
    {
        let cache_key = key.as_ref();
        let mut redis = App::global()
            .get_component::<Redis>()
            .expect("redis component don't exists");
        let cached: Result<String, RedisError> = redis.get(format!("config:{cache_key}")).await;
        Ok(match cached {
            Ok(json) => Some(serde_json::from_str(&json).context("json decode failed")?),
            Err(_err) => {
                let value = Self::find_by_key(db, key).await?.map(|m| m.value);

                match value {
                    Some(json) => Some(serde_json::from_str(&json).context("json decode failed")?),
                    None => None,
                }
            }
        })
    }
}
