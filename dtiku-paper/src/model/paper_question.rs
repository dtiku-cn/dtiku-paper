pub use super::_entities::paper_question::*;
use crate::query::question::PaperQuestionQuery;
use anyhow::Context;
use sea_orm::{
    sea_query::{IntoCondition, OnConflict},
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QuerySelect,
};
use std::collections::HashMap;

impl Entity {
    pub async fn find_by_question_id<C>(db: &C, question_id: i32) -> anyhow::Result<Vec<Model>>
    where
        C: ConnectionTrait,
    {
        Entity::find()
            .select_only()
            .columns([
                Column::PaperId,
                Column::QuestionId,
                Column::Sort,
                Column::PaperType,
            ])
            .filter(Column::QuestionId.eq(question_id))
            .all(db)
            .await
            .with_context(|| format!("paper_question::find_by_question_id({question_id}) failed"))
    }

    pub async fn find_by_question_id_in<C>(
        db: &C,
        question_ids: Vec<i32>,
    ) -> anyhow::Result<Vec<Model>>
    where
        C: ConnectionTrait,
    {
        Entity::find()
            .select_only()
            .columns([
                Column::PaperId,
                Column::QuestionId,
                Column::Sort,
                Column::PaperType,
            ])
            .filter(Column::QuestionId.is_in(question_ids))
            .all(db)
            .await
            .with_context(|| format!("paper_question::find_by_question_id_in() failed"))
    }

    pub async fn find_by_paper_id<C>(db: &C, paper_id: i32) -> anyhow::Result<Vec<Model>>
    where
        C: ConnectionTrait,
    {
        Entity::find()
            .select_only()
            .columns([
                Column::PaperId,
                Column::QuestionId,
                Column::Sort,
                Column::PaperType,
            ])
            .filter(Column::PaperId.eq(paper_id))
            .all(db)
            .await
            .with_context(|| format!("paper_question::find_by_paper_id({paper_id}) failed"))
    }

    pub async fn find_by_paper_id_and_sort_between<C>(
        db: &C,
        paper_id: i32,
        start: i16,
        end: i16,
    ) -> anyhow::Result<HashMap<i32, i16>>
    where
        C: ConnectionTrait,
    {
        Ok(Entity::find()
            .select_only()
            .column(Column::QuestionId)
            .column(Column::Sort)
            .filter(Column::PaperId.eq(paper_id).and(Column::Sort.between(start, end)))
            .into_tuple::<(i32, i16)>()
            .all(db)
            .await
            .with_context(|| format!("paper_question::find_by_paper_id_and_sort_between({paper_id}, {start}, {end}) failed"))?
            .into_iter()
            .collect())
    }

    pub async fn find_question_id_by_query<C>(
        db: &C,
        query: &PaperQuestionQuery,
    ) -> anyhow::Result<Vec<Model>>
    where
        C: ConnectionTrait,
    {
        Entity::find()
            .select_only()
            .columns([
                Column::PaperId,
                Column::QuestionId,
                Column::Sort,
                Column::PaperType,
            ])
            .filter(query.clone().into_condition())
            .all(db)
            .await
            .with_context(|| format!("paper_question::find_by_query failed"))
    }

    pub async fn find_by_paper_type_and_qid_gt<C: ConnectionTrait>(
        db: &C,
        paper_type: i16,
        qid: i32,
    ) -> anyhow::Result<Vec<i32>> {
        Entity::find()
            .select_only()
            .columns([Column::QuestionId])
            .filter(
                Column::PaperType
                    .eq(paper_type)
                    .and(Column::QuestionId.gt(qid)),
            )
            .into_tuple()
            .all(db)
            .await
            .with_context(|| {
                format!("paper_question::find_by_paper_type_and_qid_gt({paper_type},{qid}) failed")
            })
    }
}

impl ActiveModel {
    pub async fn insert_on_conflict<C>(self, db: &C) -> Result<Model, DbErr>
    where
        C: ConnectionTrait,
    {
        Entity::insert(self)
            .on_conflict(
                OnConflict::columns([Column::PaperId, Column::QuestionId])
                    .update_columns([
                        Column::Sort,
                        Column::PaperType,
                        Column::CorrectRatio,
                        Column::KeypointPath,
                    ])
                    .to_owned(),
            )
            .exec_with_returning(db)
            .await
    }
}
