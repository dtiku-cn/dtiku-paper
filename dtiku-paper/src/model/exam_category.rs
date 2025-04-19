use sea_orm::{sea_query::OnConflict, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter};

pub use super::_entities::exam_category::*;

impl Entity{
    pub async fn find_all_by_pid<C>(db: &C, pid: i16) -> Result<Vec<Model>, DbErr>
    where
        C: ConnectionTrait,
    {
        Entity::find()
            .filter(Column::Pid.eq(pid))
            .all(db)
            .await
    }
}

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
