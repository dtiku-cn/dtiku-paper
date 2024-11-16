use crate::error::Error;
pub use super::_entities::schedule_task::*;
use sea_orm::{
    sqlx::types::chrono::Local, ActiveModelBehavior, ActiveValue, ColumnTrait, ConnectionTrait,
    DbErr, EntityTrait, QueryFilter, Set,
};
use spring::{async_trait, App};
use spring_stream::Producer;

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

    async fn after_save<C>(model: Model, _db: &C, _insert: bool) -> Result<Model, DbErr>
    where
        C: ConnectionTrait,
    {
        if model.active {
            let producer = App::global()
                .get_component::<Producer>()
                .expect("stream producer component don't exists");
            let _ = producer.send_json("task", &model).await;
        }
        Ok(model)
    }
}

impl ActiveModel {
    // 乐观锁更新
    pub async fn optimistic_update<C>(mut self, db: &C) -> Result<Model, Error>
    where
        C: ConnectionTrait,
    {
        let old_version = match self.version {
            ActiveValue::Set(v) => v,
            _ => Err(Error::OptimisticLockErr(format!(
                "schedule_task version not set"
            )))?,
        };
        self.version = Set(old_version + 1);
        let am = ActiveModelBehavior::before_save(self, db, false).await?;
        let model = Entity::update(am)
            .filter(Column::Version.eq(old_version))
            .exec(db)
            .await?;
        Ok(Self::after_save(model, db, false).await?)
    }
}
