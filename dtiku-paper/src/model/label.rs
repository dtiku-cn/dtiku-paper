pub use super::_entities::label::*;
use anyhow::Context;
use sea_orm::{
    sea_query::OnConflict, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter,
};

impl ActiveModel {
    pub async fn insert_on_conflict<C: ConnectionTrait>(self, db: &C) -> Result<Model, DbErr> {
        Entity::insert(self)
            .on_conflict(
                OnConflict::columns([Column::PaperType, Column::Pid, Column::Name])
                    .do_nothing()
                    .to_owned(),
            )
            .exec_with_returning(db)
            .await
    }
}

impl Entity {
    pub async fn find_by_pid<C>(db: &C, pid: i32) -> anyhow::Result<Vec<Model>>
    where
        C: ConnectionTrait,
    {
        Entity::find()
            .filter(Column::Pid.eq(pid))
            .all(db)
            .await
            .with_context(|| format!("find_by_pid({pid}) failed"))
    }

    pub async fn find_by_exam_id_and_paper_type_and_name<C, S>(
        db: &C,
        exam_id: i16,
        paper_type: i16,
        name: S,
    ) -> anyhow::Result<Option<Model>>
    where
        C: ConnectionTrait,
        S: Into<String>,
    {
        let name = name.into();
        Entity::find()
            .filter(
                Column::ExamId
                    .eq(exam_id)
                    .and(Column::PaperType.eq(paper_type).and(Column::Name.eq(&name))),
            )
            .one(db)
            .await
            .with_context(|| {
                format!(
                    "find_by_exam_id_and_paper_type_and_name({exam_id},{paper_type},{name}) failed"
                )
            })
    }
}
