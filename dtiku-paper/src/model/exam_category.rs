use sea_orm::{
    sea_query::OnConflict, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter,
};

pub use super::_entities::exam_category::*;

impl Entity {
    pub async fn find_all_by_pid<C>(db: &C, pid: i16) -> Result<Vec<Model>, DbErr>
    where
        C: ConnectionTrait,
    {
        Entity::find().filter(Column::Pid.eq(pid)).all(db).await
    }

    pub async fn find_root_by_id<C>(db: &C, mut id: i16) -> Result<Option<Model>, DbErr>
    where
        C: ConnectionTrait,
    {
        if id <= 0 {
            return Ok(None);
        }

        loop {
            let model = Entity::find_by_id(id).one(db).await?;
            if let Some(model) = model {
                if model.pid == 0 {
                    return Ok(Some(model));
                }
                id = model.pid;
            } else {
                return Ok(model);
            }
        }
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
