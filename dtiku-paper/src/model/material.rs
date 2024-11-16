pub use super::_entities::material::*;
use super::{PaperMaterial, _entities::paper_material};
use itertools::Itertools;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DerivePartialModel, EntityTrait, FromQueryResult, QueryFilter,
};
use std::collections::HashMap;

#[derive(DerivePartialModel, FromQueryResult)]
#[sea_orm(entity = "Entity")]
pub struct Material {
    #[sea_orm(from_col = "id")]
    pub id: i32,
    #[sea_orm(from_col = "content")]
    pub content: String,
}

impl Entity {
    pub async fn find_by_paper_id<C>(db: &C, paper_id: i32) -> anyhow::Result<Vec<Material>>
    where
        C: ConnectionTrait,
    {
        let pms = PaperMaterial::find()
            .filter(paper_material::Column::PaperId.eq(paper_id))
            .all(db)
            .await?;

        let id_sort: HashMap<i32, i16> = pms.iter().map(|pm| (pm.material_id, pm.sort)).collect();

        let mids = id_sort.keys().cloned().collect_vec();

        let materials = Entity::find()
            .filter(Column::Id.is_in(mids))
            .into_partial_model::<Material>()
            .all(db)
            .await?;

        Ok(materials
            .into_iter()
            .sorted_by_key(|m| id_sort.get(&m.id))
            .collect_vec())
    }
}
