pub use super::_entities::paper::*;
use anyhow::Context;
use sea_orm::{
    sea_query::OnConflict, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, FromJsonQueryResult,
    QueryFilter,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(tag = "type")]
pub enum PaperExtra {
    #[serde(rename = "cs")]
    Chapters(Chapters),
    #[serde(rename = "ce")]
    EssayCluster(EssayCluster),
}

impl PaperExtra {
    pub fn compute_chapter(&self, num: i32) -> Option<String> {
        match self {
            Self::Chapters(cs) => cs.compute_chapter(num),
            Self::EssayCluster(_) => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct Chapters {
    pub desc: Option<String>,
    pub chapters: Vec<PaperChapter>,
}

impl Chapters {
    fn compute_chapter(&self, num: i32) -> Option<String> {
        let mut num_adder = 0;
        for c in &self.chapters {
            let prev_num_adder = num_adder;
            num_adder += c.count as i32;
            if num > prev_num_adder && num <= num_adder {
                return Some(c.name.clone());
            }
        }
        None
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct PaperChapter {
    pub name: String,
    pub desc: String,
    pub count: i16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct EssayCluster {
    pub topic: Option<String>,
    pub blocks: Vec<PaperBlock>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct PaperBlock {
    pub name: String,
    pub desc: String,
}

impl Entity {
    pub async fn find_by_label_id<C>(db: &C, label_id: i32) -> anyhow::Result<Vec<Model>>
    where
        C: ConnectionTrait,
    {
        Entity::find()
            .filter(Column::LabelId.eq(label_id))
            .all(db)
            .await
            .with_context(|| format!("find_by_label_id({label_id}) failed"))
    }
}

impl ActiveModel {
    pub async fn insert_on_conflict<C>(self, db: &C) -> Result<Model, DbErr>
    where
        C: ConnectionTrait,
    {
        Entity::insert(self)
            .on_conflict(
                OnConflict::columns([Column::LabelId, Column::Title])
                    .update_column(Column::Extra)
                    .to_owned(),
            )
            .exec_with_returning(db)
            .await
    }
}
