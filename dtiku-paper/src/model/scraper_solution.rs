pub use super::_entities::scraper_solution::*;
use sea_orm::{ActiveModelBehavior, ConnectionTrait};

impl ActiveModelBehavior for ActiveModel {}

impl ActiveModel {
    pub async fn insert_on_conflict<C>(self, _db: &C) -> anyhow::Result<Model>
    where
        C: ConnectionTrait,
    {
        todo!()
    }
}
