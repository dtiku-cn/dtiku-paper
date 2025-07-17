pub use super::_entities::idiom_ref::*;
use sea_orm::{sea_query::OnConflict, ConnectionTrait, DbErr, EntityTrait as _};

impl ActiveModel {
    pub async fn insert_on_conflict<C: ConnectionTrait>(self, db: &C) -> Result<Model, DbErr> {
        Entity::insert(self)
            .on_conflict(
                OnConflict::columns([
                    Column::Ty,
                    Column::LabelId,
                    Column::IdiomId,
                    Column::PaperId,
                    Column::QuestionId,
                ])
                .update_columns([Column::ExamId, Column::PaperType])
                .to_owned(),
            )
            .exec_with_returning(db)
            .await
    }
}
