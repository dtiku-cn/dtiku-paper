use anyhow::Context;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

pub use super::_entities::paper_question::*;

impl Entity {
    pub async fn find_by_paper_id<C>(db: &C, paper_id: i32) -> anyhow::Result<Vec<Model>>
    where
        C: ConnectionTrait,
    {
        Entity::find()
            .filter(Column::PaperId.eq(paper_id))
            .all(db)
            .await
            .with_context(|| format!("paper_question::find_by_paper_id({paper_id}) failed"))
    }
}
