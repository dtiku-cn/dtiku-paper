use std::collections::HashMap;

use anyhow::Context;
use itertools::Itertools;
use sea_orm::{
    sea_query::OnConflict, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter,
};

pub use super::_entities::question_material::*;

impl Entity {
    pub async fn find_by_qid<C>(db: &C, qid: i32) -> anyhow::Result<Vec<i32>>
    where
        C: ConnectionTrait,
    {
        Ok(Entity::find()
            .filter(Column::QuestionId.eq(qid))
            .all(db)
            .await
            .context("question_material::find_by_qid() failed")?
            .into_iter()
            .map(|r| r.material_id)
            .collect_vec())
    }

    pub async fn find_by_qids<C>(db: &C, qids: Vec<i32>) -> anyhow::Result<HashMap<i32, Vec<i32>>>
    where
        C: ConnectionTrait,
    {
        let rows = Entity::find()
            .filter(Column::QuestionId.is_in(qids))
            .all(db)
            .await
            .context("question_material::find_by_qids() failed")?;
        Ok(rows
            .into_iter()
            .map(|r| (r.question_id, r.material_id))
            .into_group_map())
    }
}

impl ActiveModel {
    pub async fn insert_on_conflict<C>(self, db: &C) -> Result<Model, DbErr>
    where
        C: ConnectionTrait,
    {
        Entity::insert(self)
            .on_conflict(
                OnConflict::columns([Column::QuestionId, Column::MaterialId])
                    .do_nothing()
                    .to_owned(),
            )
            .exec_with_returning(db)
            .await
    }
}
