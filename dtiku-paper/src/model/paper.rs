pub use super::_entities::paper::*;
use crate::query::paper::ListPaperQuery;
use anyhow::Context;
use sea_orm::{
    sea_query::OnConflict, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, FromJsonQueryResult,
    QueryFilter, QueryOrder, QuerySelect,
};
use serde::{Deserialize, Serialize};
use spring_sea_orm::pagination::{Page, PaginationExt};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(tag = "type")]
pub enum PaperExtra {
    #[serde(rename = "cs")]
    Chapters(Chapters),
    #[serde(rename = "ce")]
    EssayCluster(EssayCluster),
}

impl PaperExtra {
    pub fn is_essay(&self) -> bool {
        match self {
            Self::Chapters(_) => false,
            Self::EssayCluster(_) => true,
        }
    }

    pub fn compute_chapter_name(&self, num: i32) -> Option<String> {
        self.compute_chapter(num, false).map(|c| c.name)
    }

    pub fn compute_block(&self, index: usize) -> Option<PaperBlock> {
        match self {
            Self::EssayCluster(ec) => ec.blocks.get(index).cloned(),
            Self::Chapters(_) => None,
        }
    }

    pub fn block_count(&self) -> usize {
        match self {
            Self::EssayCluster(ec) => ec.blocks.len(),
            Self::Chapters(_) => 0,
        }
    }

    pub fn compute_chapter(&self, num: i32, only_first: bool) -> Option<PaperChapter> {
        match self {
            Self::Chapters(cs) => cs.compute_chapter(num, only_first),
            Self::EssayCluster(_) => None,
        }
    }

    pub fn topic(&self) -> Option<String> {
        match self {
            Self::Chapters(_) => None,
            Self::EssayCluster(ec) => ec.topic.clone(),
        }
    }

    pub fn compute_question_range(&self, pattern: &str) -> Option<(i16, i16)> {
        match self {
            Self::Chapters(cs) => {
                let mut start = 1;
                for c in &cs.chapters {
                    if c.name.contains(pattern) {
                        return Some((start, start + c.count - 1));
                    } else {
                        start += c.count;
                    }
                }
                None
            }
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
    fn compute_chapter(&self, num: i32, only_first: bool) -> Option<PaperChapter> {
        let mut num_adder = 0;
        for c in &self.chapters {
            let prev_num_adder = num_adder;
            num_adder += c.count as i32;
            if only_first {
                if num == prev_num_adder + 1 {
                    return Some(c.clone());
                }
            } else if num > prev_num_adder && num <= num_adder {
                return Some(c.clone());
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
    pub async fn find_by_ids<C>(db: &C, ids: Vec<i32>) -> anyhow::Result<Vec<Model>>
    where
        C: ConnectionTrait,
    {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        Entity::find()
            .filter(Column::Id.is_in(ids))
            .all(db)
            .await
            .context("paper::find_by_ids() failed")
    }

    pub async fn find_by_query<C>(db: &C, query: &ListPaperQuery) -> anyhow::Result<Page<Model>>
    where
        C: ConnectionTrait,
    {
        Entity::find()
            .filter(
                Column::LabelId
                    .eq(query.label_id)
                    .and(Column::PaperType.eq(query.paper_type)),
            )
            .order_by_desc(Column::Year)
            .order_by_desc(Column::Id)
            .page(db, &query.page)
            .await
            .with_context(|| format!("find_by_query({query:?}) failed"))
    }

    pub async fn find_by_paper_type_and_id_gt<C>(
        db: &C,
        paper_id: i16,
        last_id: i32,
    ) -> anyhow::Result<Vec<Model>>
    where
        C: ConnectionTrait,
    {
        Entity::find()
            .filter(Column::PaperType.eq(paper_id).and(Column::Id.gt(last_id)))
            .order_by_asc(Column::Id)
            .limit(50)
            .all(db)
            .await
            .with_context(|| format!("find_by_paper_type_and_id_gt({paper_id},{last_id}) failed"))
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
