pub use super::_entities::key_point::*;
use sea_orm::{
    sea_query::OnConflict, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter,
};

impl Entity {
    pub async fn find_by_paper_type_and_name<C: ConnectionTrait>(
        db: &C,
        paper_type: i16,
        name: &str,
    ) -> Result<Option<Model>, DbErr> {
        Entity::find()
            .filter(Column::PaperType.eq(paper_type).and(Column::Name.eq(name)))
            .one(db)
            .await
    }

    pub async fn query_common_keypoint_path<C: ConnectionTrait>(
        db: &C,
        keypoint_ids: &[i32],
    ) -> Result<Option<String>, DbErr> {
        todo!()
    }
}

impl ActiveModel {
    pub async fn insert_on_conflict<C: ConnectionTrait>(self, db: &C) -> Result<Model, DbErr> {
        Entity::insert(self)
            .on_conflict(
                OnConflict::columns([Column::Pid, Column::Name])
                    .update_columns([Column::ExamId, Column::PaperType])
                    .to_owned(),
            )
            .exec_with_returning(db)
            .await
    }
}
