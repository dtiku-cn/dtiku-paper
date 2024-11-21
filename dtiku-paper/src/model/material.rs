pub use super::_entities::material::*;
use super::{PaperMaterial, _entities::paper_material};
use itertools::Itertools;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DerivePartialModel, EntityTrait, FromQueryResult, QueryFilter,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

pub struct Material {
    pub id: i32,
    pub content: String,
    pub extra: Vec<MaterialExtra>,
}

#[derive(DerivePartialModel, FromQueryResult)]
#[sea_orm(entity = "Entity")]
struct MaterialSelect {
    #[sea_orm(from_col = "id")]
    id: i32,
    #[sea_orm(from_col = "content")]
    content: String,
    #[sea_orm(from_col = "extra")]
    extra: Value,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MaterialExtra {
    #[serde(rename = "explain")]
    MaterialExplain(String),
    #[serde(rename = "dict")]
    Dictionary(String),
    #[serde(rename = "translation")]
    Translation(String),
    #[serde(rename = "audio")]
    Audio(String),
    #[serde(rename = "transcript")]
    Transcript(String),
}

impl TryFrom<MaterialSelect> for Material {
    type Error = anyhow::Error;

    fn try_from(value: MaterialSelect) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.id,
            content: value.content,
            extra: serde_json::from_value(value.extra)?,
        })
    }
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
            .into_partial_model::<MaterialSelect>()
            .all(db)
            .await?;

        materials
            .into_iter()
            .sorted_by_key(|m| id_sort.get(&m.id))
            .map(|m| m.try_into())
            .collect()
    }
}
