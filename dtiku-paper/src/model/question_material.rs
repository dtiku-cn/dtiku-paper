use std::collections::HashMap;

use anyhow::Context;
use itertools::Itertools;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

pub use super::_entities::question_material::*;

impl Entity {
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
