use sea_orm::{sea_query::OnConflict, ConnectionTrait, DbErr, EntityTrait};

pub use super::_entities::question_key_point::*;

impl ActiveModel {
    pub async fn insert_on_conflict<C>(self, db: &C) -> Result<Model, DbErr>
    where
        C: ConnectionTrait,
    {
        Entity::insert(self)
            .on_conflict(
                OnConflict::columns([Column::QuestionId, Column::KeyPointId])
                    .do_nothing()
                    .to_owned(),
            )
            .exec_with_returning(db)
            .await
    }
}
