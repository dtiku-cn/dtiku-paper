pub use super::_entities::material::*;
use super::{PaperMaterial, _entities::paper_material};
use itertools::Itertools;
use sea_orm::{
    sea_query::OnConflict, ColumnTrait, ConnectionTrait, DbErr, DerivePartialModel, EntityTrait,
    FromJsonQueryResult, FromQueryResult, QueryFilter,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub struct Material {
    pub id: i32,
    pub content: String,
    pub extra: Vec<MaterialExtra>,
    pub num: i16,
}

#[derive(DerivePartialModel, FromQueryResult)]
#[sea_orm(entity = "Entity")]
struct MaterialSelect {
    #[sea_orm(from_col = "id")]
    id: i32,
    #[sea_orm(from_col = "content")]
    content: String,
    #[sea_orm(from_col = "extra")]
    extra: Vec<MaterialExtra>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
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

impl MaterialSelect {
    fn with_num(self, num_map: &HashMap<i32, i16>) -> Material {
        Material {
            id: self.id,
            content: self.content,
            extra: self.extra,
            num: num_map.get(&self.id).cloned().unwrap_or_default(),
        }
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

        Ok(materials
            .into_iter()
            .map(|m| m.with_num(&id_sort))
            .sorted_by_key(|m| m.num)
            .collect())
    }
}

impl ActiveModel {
    pub async fn insert_on_conflict<C>(self, db: &C) -> Result<Model, DbErr>
    where
        C: ConnectionTrait,
    {
        Entity::insert(self)
            .on_conflict(
                OnConflict::columns([Column::Id])
                    .update_columns([Column::Content, Column::Extra])
                    .to_owned(),
            )
            .exec_with_returning(db)
            .await
    }
}
