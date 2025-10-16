pub use super::_entities::idiom::*;
use crate::{
    domain::{IdiomRefStatsWithoutLabel, IdiomStats},
    model::{idiom_ref_stats, IdiomRefStats},
};
use anyhow::Context as _;
use itertools::Itertools;
use sea_orm::{
    prelude::Expr, sea_query::OnConflict, sqlx::types::chrono::Local, ActiveModelBehavior,
    ActiveValue::Set, ColumnTrait, ConnectionTrait, DbErr, DerivePartialModel, EntityTrait as _,
    FromJsonQueryResult, FromQueryResult, QueryFilter, QueryOrder, QuerySelect as _,
};
use serde::{Deserialize, Serialize};
use spring::async_trait;
use std::collections::HashMap;

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct BasicExplain {
    pub baobian: String,
    pub definition: String,
}

impl Into<BasicExplain> for &IdiomExplainEntry {
    fn into(self) -> BasicExplain {
        match &self {
            IdiomExplainEntry::Idiom(IdiomEntry {
                baobian,
                definition_info,
                ..
            }) => BasicExplain {
                baobian: baobian.clone(),
                definition: definition_info.as_ref().unwrap().definition.clone(),
            },
            IdiomExplainEntry::Term(TermEntry {
                baobian,
                baike_info,
                ..
            }) => BasicExplain {
                baobian: baobian.clone(),
                definition: baike_info.as_ref().unwrap().baike_mean.clone(),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(tag = "type", content = "data")]
pub enum IdiomExplainEntry {
    #[serde(rename = "idiom")]
    Idiom(IdiomEntry),

    #[serde(rename = "term")]
    Term(TermEntry),
}
impl IdiomExplainEntry {
    pub(crate) fn jyc(&self) -> Vec<&String> {
        todo!()
    }
    pub(crate) fn fyc(&self) -> Vec<&String> {
        todo!()
    }
}

//
// ---------------- idiom ----------------
//
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct IdiomEntry {
    pub idiom_version: i32,
    pub name: String,
    pub imgs: Vec<String>,
    pub definition_info: Option<DefinitionInfo>,
    pub liju: Vec<Liju>,
    pub story: Vec<String>,
    pub antonym: Vec<WordRef>,
    pub synonyms: Vec<WordRef>,
    pub tongyiyixing: Vec<WordRef>,
    pub chu_chu: Vec<Citation>,
    pub yin_zheng: Vec<Citation>,
    pub baobian: String,
    pub structure: String,
    pub pinyin: String,
    pub voice: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionInfo {
    pub definition: String,
    pub similar_definition: String,
    pub ancient_definition: String,
    pub modern_definition: String,
    pub detail_means: Vec<WordDefinition>,
    pub usage_tips: Vec<String>,
    pub yicuodian: Vec<String>,
    pub baobian: String,
    pub word_formation: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct WordDefinition {
    pub word: String,
    pub definition: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct Liju {
    pub name: String,
    pub show_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct WordRef {
    pub name: String,
    #[serde(default)]
    pub is_click: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct Citation {
    pub source_chapter: String,
    pub source: String,
    pub dynasty: String,
    pub cite_original_text: String,
    pub author: String,
}

//
// ---------------- term ----------------
//
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct TermEntry {
    pub term_version: i32,
    pub imgs: Vec<String>,
    pub comprehensive_definition: Vec<ComprehensiveDefinition>,
    pub modifier: Vec<WordRef>,
    pub baike_info: Option<BaikeInfo>,
    pub baobian: String,
    pub structure: String,
    pub term_style: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct ComprehensiveDefinition {
    pub pinyin: String,
    pub voice: String,
    pub basic_definition: Vec<BasicDefinition>,
    pub detail_definition: Vec<DetailDefinition>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct BasicDefinition {
    pub definition: String,
    pub synonyms: Vec<WordRef>,
    pub antonym: Vec<WordRef>,
    pub baobian: String,
    pub zuci: Vec<WordRef>,
    pub liju: Vec<Liju>,
    pub shiyongchangjing: String,
    pub grammar_struct: Vec<String>,
    pub cixing: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct DetailDefinition {
    pub definition: String,
    pub baobian: String,
    pub liju: Vec<Liju>,
    pub cite_list: Vec<Citation>,
    pub cixing: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct BaikeInfo {
    pub baike_mean: String,
    pub baike_url: String,
}

#[derive(Clone, Debug, DerivePartialModel, FromQueryResult, Serialize, Deserialize)]
#[sea_orm(entity = "Entity")]
pub struct BriefIdiom {
    #[sea_orm(from_col = "id")]
    pub id: i32,
    #[sea_orm(from_col = "text")]
    pub text: String,
    #[sea_orm(from_col = "explain")]
    pub explain: BasicExplain,
}

#[async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(mut self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if insert {
            self.created = Set(Local::now().naive_local());
        }
        self.modified = Set(Local::now().naive_local());
        Ok(self)
    }
}

impl ActiveModel {
    pub async fn insert_on_conflict<C: ConnectionTrait>(self, db: &C) -> Result<Model, DbErr> {
        let am = ActiveModelBehavior::before_save(self, db, true).await?;
        let model = Entity::insert(am)
            .on_conflict(
                OnConflict::columns([Column::Text])
                    .update_columns([Column::Ty, Column::Content, Column::Modified])
                    .to_owned(),
            )
            .exec_with_returning(db)
            .await?;
        Self::after_save(model, db, true).await
    }
}

impl Entity {
    pub async fn find_by_text<C: ConnectionTrait>(
        db: &C,
        text: &str,
    ) -> anyhow::Result<Option<Model>> {
        Ok(Self::find().filter(Column::Text.eq(text)).one(db).await?)
    }

    pub async fn find_by_texts<C: ConnectionTrait>(
        db: &C,
        texts: Vec<&String>,
        labels: &Vec<i32>,
    ) -> anyhow::Result<Vec<IdiomStats>> {
        let brief = Entity::find()
            .select_only()
            .columns([Column::Id, Column::Text, Column::Explain])
            .filter(Column::Text.is_in(texts))
            .into_partial_model::<BriefIdiom>()
            .all(db)
            .await
            .context("Idiom::find_by_texts() failed")?;

        let id_idiom_map: HashMap<i32, _> = brief.into_iter().map(|i| (i.id, i)).collect();
        let idiom_ids = id_idiom_map.keys().cloned().collect_vec();

        let mut ref_stats_filter = idiom_ref_stats::Column::IdiomId.is_in(idiom_ids);
        if !labels.is_empty() {
            ref_stats_filter =
                ref_stats_filter.and(idiom_ref_stats::Column::LabelId.is_in(labels.clone()));
        }
        let stats = IdiomRefStats::find()
            .select_only()
            .column(idiom_ref_stats::Column::IdiomId)
            .column_as(Expr::cust("SUM(question_count)::BIGINT"), "question_count")
            .column_as(Expr::cust("SUM(paper_count)::BIGINT"), "paper_count")
            .filter(ref_stats_filter)
            .group_by(idiom_ref_stats::Column::IdiomId)
            .order_by_desc(Expr::col("question_count"))
            .into_model::<IdiomRefStatsWithoutLabel>()
            .all(db)
            .await
            .context("IdiomRefStats::idiom_stats() failed")?;

        Ok(stats
            .into_iter()
            .map(|s| IdiomStats::from_brief(id_idiom_map.get(&s.idiom_id), s))
            .collect_vec())
    }

    pub async fn find_brief_in_ids<C: ConnectionTrait>(
        db: &C,
        idiom_ids: Vec<i32>,
    ) -> anyhow::Result<Vec<BriefIdiom>> {
        Entity::find()
            .select_only()
            .columns([Column::Id, Column::Text, Column::Explain])
            .filter(Column::Id.is_in(idiom_ids))
            .into_partial_model::<BriefIdiom>()
            .all(db)
            .await
            .context("Idiom::find_breaf_in_ids() failed")
    }
}
