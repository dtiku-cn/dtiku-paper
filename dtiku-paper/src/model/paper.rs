pub use super::_entities::paper::*;
use anyhow::Context;
use sea_orm::{
    ActiveModelBehavior, ColumnTrait, ConnectionTrait, DerivePartialModel, EntityTrait,
    FromQueryResult, QueryFilter,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::ops::Range;

impl ActiveModelBehavior for ActiveModel {}

pub struct Paper {
    pub id: i32,
    pub title: String,
    pub descrp: Option<String>,
    pub label_id: i32,
    pub extra: PaperExtra,
}

#[derive(DerivePartialModel, FromQueryResult)]
#[sea_orm(entity = "Entity")]
struct PaperSelect {
    #[sea_orm(from_col = "id")]
    id: i32,
    #[sea_orm(from_col = "title")]
    title: String,
    #[sea_orm(from_col = "descrp")]
    pub descrp: Option<String>,
    #[sea_orm(from_col = "label_id")]
    pub label_id: i32,
    #[sea_orm(from_col = "extra")]
    extra: Value,
}

impl TryFrom<PaperSelect> for Paper {
    type Error = anyhow::Error;

    fn try_from(value: PaperSelect) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.id,
            title: value.title,
            descrp: value.descrp,
            label_id: value.label_id,
            extra: serde_json::from_value(value.extra)?,
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PaperExtra {
    #[serde(rename = "cs")]
    Chapter(Vec<PaperChapter>),
    #[serde(rename = "ce")]
    EssayCluster(EssayCluster),
}

#[derive(Serialize, Deserialize)]
pub struct PaperChapter {
    pub name: String,
    pub desc: String,
    pub range: Range<i16>,
}

#[derive(Serialize, Deserialize)]
pub struct EssayCluster {
    pub topic: Option<String>,
    pub blocks: Vec<PaperBlock>,
}

#[derive(Serialize, Deserialize)]
pub struct PaperBlock {
    pub name: String,
    pub desc: String,
}

impl Entity {
    pub async fn find_by_label_id<C>(db: &C, label_id: i32) -> anyhow::Result<Vec<Paper>>
    where
        C: ConnectionTrait,
    {
        Entity::find()
            .filter(Column::LabelId.eq(label_id))
            .into_partial_model::<PaperSelect>()
            .all(db)
            .await
            .with_context(|| format!("find_by_label_id(${label_id}) failed"))?
            .into_iter()
            .map(|p| p.try_into())
            .collect()
    }
}
