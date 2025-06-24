pub use super::_entities::paper_material::*;
use sea_orm::{sea_query::OnConflict, ConnectionTrait, DbErr, EntityTrait};

impl ActiveModel {
    pub async fn insert_on_conflict<C>(self, db: &C) -> Result<Model, DbErr>
    where
        C: ConnectionTrait,
    {
        Entity::insert(self)
            .on_conflict(
                OnConflict::columns([Column::PaperId, Column::MaterialId])
                    .update_columns([Column::Sort])
                    .to_owned(),
            )
            .exec_with_returning(db)
            .await
    }
}
