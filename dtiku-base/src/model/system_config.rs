pub use super::_entities::system_config::*;
use super::enums::SystemConfigKey;
use anyhow::Context;
use sea_orm::{
    sqlx::types::chrono::Local, ActiveModelBehavior, ColumnTrait, ConnectionTrait, DbErr,
    EntityTrait, QueryFilter, Set,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use spring::{async_trait, plugin::ComponentRegistry, App};
use spring_redis::{cache, redis::AsyncCommands, Redis};

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
        let config_key = self.key.as_ref();
        let mut redis = App::global().get_expect_component::<Redis>();
        let _: () = redis.del(format!("config:{config_key:?}")).await.unwrap();
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

    pub async fn decode_cached_value<C, T>(
        db: &C,
        key: SystemConfigKey,
    ) -> anyhow::Result<Option<T>>
    where
        C: ConnectionTrait,
        T: DeserializeOwned + Default,
    {
        let value = Self::find_cached_value_by_key(db, key).await?;
        let v = match value {
            Some(v) => match v {
                Value::Null => None,
                Value::String(s) => {
                    if s.is_empty() {
                        None
                    } else {
                        serde_json::from_str(&format!("\"{s}\""))
                            .with_context(|| format!("parse json failed for {key:?}"))?
                    }
                }
                _ => Some(
                    serde_json::from_value(v)
                        .with_context(|| format!("parse json failed for {key:?}"))?,
                ),
            },
            None => None,
        };
        Ok(v)
    }

    #[cache("config:{key:?}", expire = 86400)]
    pub async fn find_cached_value_by_key<C>(
        db: &C,
        key: SystemConfigKey,
    ) -> anyhow::Result<Option<Value>>
    where
        C: ConnectionTrait,
    {
        Entity::find()
            .filter(Column::Key.eq(key))
            .one(db)
            .await
            .with_context(|| format!("system_config::find_by_key({key:?}) failed"))
            .map(|om| om.map(|m| m.value))
    }

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
