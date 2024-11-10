pub use super::_entities::schedule_task::*;
use sea_orm::{sqlx::types::chrono::Local, ActiveModelBehavior, ConnectionTrait, DbErr, Set};
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
            let json = serde_json::to_string(&model).expect("json serde failed");

            let producer = App::global()
                .get_component::<Producer>()
                .expect("stream producer component don't exists");
            let _ = producer.send_to("task", json).await;
        }
        Ok(model)
    }
}
