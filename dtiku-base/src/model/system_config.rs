pub use super::_entities::system_config::*;
use super::enums::SystemConfigKey;
use anyhow::Context;
use sea_orm::{
    sqlx::types::chrono::Local, ActiveModelBehavior, ColumnTrait, ConnectionTrait, DbErr,
    EntityTrait, QueryFilter, QuerySelect, Set,
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

    pub async fn find_value_by_key<C, T>(db: &C, key: SystemConfigKey) -> anyhow::Result<Option<T>>
    where
        C: ConnectionTrait,
        T: DeserializeOwned,
    {
        let key = key.as_ref();
        let mut redis = App::global()
            .get_component::<Redis>()
            .expect("redis component don't exists");
        let cached: Result<String, RedisError> = redis.get(format!("config:{key}")).await;
        Ok(match cached {
            Ok(json) => Some(serde_json::from_str(&json).context("json decode failed")?),
            Err(_err) => {
                let json: Option<String> = Entity::find()
                    .select_only()
                    .column(Column::Value)
                    .filter(Column::Key.eq(key))
                    .into_tuple()
                    .one(db)
                    .await?;

                match json {
                    Some(json) => Some(serde_json::from_str(&json).context("json decode failed")?),
                    None => None,
                }
            }
        })
    }
}
