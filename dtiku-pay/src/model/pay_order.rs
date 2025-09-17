pub use super::_entities::pay_order::*;
use anyhow::Context;
use sea_orm::{
    prelude::DateTime, sqlx::types::chrono::Local, ActiveModelBehavior, ActiveValue::Set,
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter,
};
use spring::{async_trait, plugin::ComponentRegistry, App};
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

    async fn after_save<C>(model: Model, _db: &C, insert: bool) -> Result<Model, DbErr>
    where
        C: ConnectionTrait,
    {
        if insert {
            let producer = App::global().get_expect_component::<Producer>();
            let _ = producer.send_json("pay_order", &model).await;
        }
        Ok(model)
    }
}

impl Entity {
    pub async fn find_wait_confirm_after<C: ConnectionTrait>(
        db: &C,
        time: DateTime,
    ) -> anyhow::Result<Vec<Model>> {
        Entity::find()
            .filter(Column::Confirm.is_null().and(Column::Created.gt(time)))
            .all(db)
            .await
            .with_context(|| format!("find_wait_confirm({time:?}) failed"))
    }
}
