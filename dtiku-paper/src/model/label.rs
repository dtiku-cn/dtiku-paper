pub use super::_entities::label::*;
use super::query::label::LabelQuery;
use anyhow::Context;
use sea_orm::{
    sea_query::OnConflict, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect,
};
use spring_redis::cache;

impl ActiveModel {
    pub async fn insert_on_conflict<C: ConnectionTrait>(self, db: &C) -> Result<Model, DbErr> {
        Entity::insert(self)
            .on_conflict(
                OnConflict::columns([Column::PaperType, Column::Pid, Column::Name])
                    .update_column(Column::ExamId)
                    .to_owned(),
            )
            .exec_with_returning(db)
            .await
    }
}

impl Entity {
    #[cache("label:{id}", expire = 86400)]
    pub async fn find_by_id_with_cache<C>(db: &C, id: i32) -> anyhow::Result<Option<Model>>
    where
        C: ConnectionTrait,
    {
        Entity::find_by_id(id)
            .one(db)
            .await
            .with_context(|| format!("Label::find_by_id({id}) failed"))
    }

    pub async fn find_all_by_query<C>(db: &C, query: LabelQuery) -> anyhow::Result<Vec<Model>>
    where
        C: ConnectionTrait,
    {
        Entity::find()
            .filter(query.clone())
            .order_by_asc(Column::Id)
            .all(db)
            .await
            .with_context(|| format!("find_all_by_query({query:?}) failed"))
    }

    pub async fn find_by_exam_id_and_paper_type_and_name<C: ConnectionTrait, S: Into<String>>(
        db: &C,
        exam_id: i16,
        paper_type: i16,
        name: S,
    ) -> anyhow::Result<Option<Model>> {
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

    pub async fn find_by_exam_id_and_paper_type<C: ConnectionTrait>(
        db: &C,
        exam_id: i16,
        paper_type: i16,
    ) -> anyhow::Result<Option<Model>> {
        Entity::find()
            .filter(
                Column::ExamId
                    .eq(exam_id)
                    .and(Column::PaperType.eq(paper_type)),
            )
            .limit(1)
            .one(db)
            .await
            .with_context(|| {
                format!("find_by_exam_id_and_paper_type({exam_id},{paper_type}) failed")
            })
    }

    #[cache("label:hidden:{paper_type}", expire = 86400)]
    pub async fn find_hidden_label_id_by_paper_type<C: ConnectionTrait>(
        db: &C,
        paper_type: i16,
    ) -> anyhow::Result<Vec<i32>> {
        Entity::find()
            .select_only()
            .column(Column::Id)
            .filter(
                Column::PaperType
                    .eq(paper_type)
                    .and(Column::Hidden.eq(true)),
            )
            .into_tuple()
            .all(db)
            .await
            .with_context(|| format!("find_hidden_label_id_by_paper_type({paper_type}) failed"))
    }

    pub async fn find_by_paper_type_and_pids<C: ConnectionTrait>(
        db: &C,
        paper_type: i16,
        pids: Vec<i32>,
    ) -> anyhow::Result<Vec<Model>> {
        Entity::find()
            .filter(
                Column::PaperType
                    .eq(paper_type)
                    .and(Column::Hidden.eq(false))
                    .and(Column::Pid.is_in(pids)),
            )
            .all(db)
            .await
            .context("find_by_paper_type_and_pids() failed")
    }
}
