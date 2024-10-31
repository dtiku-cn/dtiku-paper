pub use super::_entities::label::*;
use anyhow::Context;
use sea_orm::{ActiveModelBehavior, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

impl ActiveModelBehavior for ActiveModel {}

impl Entity {
    pub async fn find_by_pid<C>(db: &C, pid: i32) -> anyhow::Result<Vec<Model>>
    where
        C: ConnectionTrait,
    {
        Entity::find()
            .filter(Column::Pid.eq(pid))
            .all(db)
            .await
            .with_context(|| format!("query label for pid#{pid}"))
    }
}
