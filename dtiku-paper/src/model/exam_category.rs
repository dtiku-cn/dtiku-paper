use sea_orm::{sea_query::OnConflict, ConnectionTrait, DbErr, EntityTrait};

pub use super::_entities::exam_category::*;

impl ActiveModel {
    pub async fn insert_on_conflict<C: ConnectionTrait>(self, db: &C) -> Result<Model, DbErr> {
        Entity::insert(self)
            .on_conflict(
                OnConflict::columns([Column::Pid, Column::Prefix])
                    .update_column(Column::Name)
                    .to_owned(),
            )
            .exec_with_returning(db)
            .await
    }
}
