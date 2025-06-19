pub use super::_entities::paper_question::*;
use crate::query::question::PaperQuestionQuery;
use anyhow::Context;
use sea_orm::{
    sea_query::OnConflict, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter,
    QuerySelect,
};

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

    pub async fn find_by_query<C>(db: &C, query: &PaperQuestionQuery) -> anyhow::Result<Vec<Model>>
    where
        C: ConnectionTrait,
    {
        Entity::find()
            .select_only()
            .column(Column::QuestionId)
            .filter(
                Column::PaperType
                    .eq(query.paper_type)
                    .and(Column::PaperId.is_in(query.paper_ids.clone()))
                    .and(
                        Column::CorrectRatio
                            .between(query.correct_ratio_start, query.correct_ratio_end),
                    ),
            )
            .all(db)
            .await
            .with_context(|| format!("paper_question::find_by_query failed"))
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
