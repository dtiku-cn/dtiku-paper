pub use super::_entities::schedule_task::*;
use crate::error::Error;
use crate::model::enums::ScheduleTaskType;
use crate::model::schedule_task::Column::Ty;
use anyhow::Context;
use sea_orm::{
    prelude::DateTime, sqlx::types::chrono::Local, ActiveModelBehavior, ActiveValue, ColumnTrait,
    ConnectionTrait, DbErr, EntityTrait, QueryFilter, Set,
};
use serde::{Deserialize, Serialize};
use spring::async_trait;

#[derive(Debug, Serialize, Deserialize)]
pub struct Progress<T> {
    pub name: String,
    pub current: T,
    pub total: T,
}

impl Progress<i64> {
    pub fn increase(&mut self, delta: i64) -> bool {
        let old_percent = self.current * 100 / self.total;
        self.current += delta;
        let new_percent = self.current * 100 / self.total;
        old_percent != new_percent
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskInstance {
    id: String,
    start_time: DateTime,
    end_time: Option<DateTime>,
}

impl Default for TaskInstance {
    fn default() -> Self {
        let now = Local::now().naive_local();
        Self {
            id: now
                .format("%Y%m%d%H%M%S")
                .to_string()
                .parse()
                .expect("task instance id format parse failed"),
            start_time: now,
            end_time: None,
        }
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

impl ActiveModel {
    // 乐观锁更新
    pub async fn optimistic_update<C>(mut self, db: &C) -> Result<Model, Error>
    where
        C: ConnectionTrait,
    {
        let old_version = match self.version {
            ActiveValue::Set(v) => v - 1,
            ActiveValue::Unchanged(v) => v,
            _ => Err(Error::OptimisticLockErr(
                "schedule_task version not set".to_string(),
            ))?,
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

impl Model {
    pub async fn update_progress<T, C>(
        &self,
        progress: &Progress<T>,
        db: &C,
    ) -> anyhow::Result<Model>
    where
        T: Serialize,
        C: ConnectionTrait,
    {
        let model = ActiveModel {
            id: Set(self.id),
            version: Set(self.version + 1),
            context: Set(serde_json::to_value(progress)?),
            ..Default::default()
        }
        .optimistic_update(db)
        .await
        .context("update progress failed")?;
        Ok(model)
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
            .context("find all task failed")?;

        Ok(all)
    }

    pub async fn find_by_type<C>(db: &C, ty: ScheduleTaskType) -> anyhow::Result<Option<Model>>
    where
        C: ConnectionTrait,
    {
        Entity::find()
            .filter(Ty.eq(ty))
            .one(db)
            .await
            .with_context(|| format!("find_by_type({ty:?}) failed"))
    }
}
