pub use super::_entities::question_key_point_stats::*;
use anyhow::Context;
use sea_orm::{
    prelude::Expr, ColumnTrait, ConnectionTrait, EntityTrait, FromQueryResult, QueryFilter,
    QuerySelect,
};

#[derive(Debug, FromQueryResult)]
pub struct KeyPointSummary {
    pub key_point_id: i32,
    pub total_questions: i64,
}

impl Entity {
    pub async fn stats_by_key_point_ids<C: ConnectionTrait>(
        db: &C,
        key_point_ids: Vec<i32>,
    ) -> anyhow::Result<Vec<KeyPointSummary>> {
        Entity::find()
            .select_only()
            .column(Column::KeyPointId)
            .column_as(
                Expr::cust("SUM(question_count)::BIGINT"), 
                "total_questions"
            )
            .filter(Column::KeyPointId.is_in(key_point_ids))
            .group_by(Column::KeyPointId)
            .into_model::<KeyPointSummary>()
            .all(db)
            .await
            .context("Failed to find question key point stats by key point IDs")
    }
}
