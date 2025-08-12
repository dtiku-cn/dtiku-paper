pub use super::_entities::material::*;
use super::{PaperMaterial, _entities::paper_material};
use crate::{
    model::{assets, QuestionMaterial, SrcType},
    util::html,
};
use anyhow::{anyhow, Context};
use gaoya::simhash::{SimHash, SimSipHasher128};
use itertools::Itertools;
use scraper::Html;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DbBackend, EntityTrait,
    FromJsonQueryResult, FromQueryResult, QueryFilter, Statement,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Material {
    pub id: i32,
    pub content: String,
    pub extra: Vec<MaterialExtra>,
    pub num: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(tag = "type")]
pub enum MaterialExtra {
    #[serde(rename = "explain")]
    MaterialExplain { value: String },
    #[serde(rename = "dict")]
    Dictionary { value: String },
    #[serde(rename = "translation")]
    Translation { value: String },
    #[serde(rename = "audio")]
    Audio { value: String },
    #[serde(rename = "transcript")]
    Transcript { value: String },
}

impl Model {
    fn with_num(self, num_map: &HashMap<i32, i16>) -> Material {
        Material {
            id: self.id,
            content: self.content,
            extra: self.extra,
            num: num_map.get(&self.id).cloned().unwrap_or_default() as usize,
        }
    }
}

impl Entity {
    pub async fn find_by_ids<C>(db: &C, material_ids: Vec<i32>) -> anyhow::Result<Vec<Model>>
    where
        C: ConnectionTrait,
    {
        Entity::find()
            .filter(Column::Id.is_in(material_ids))
            .all(db)
            .await
            .context("find material failed")
    }

    pub async fn find_by_qid<C>(db: &C, question_id: i32) -> anyhow::Result<Vec<Model>>
    where
        C: ConnectionTrait,
    {
        let material_ids = QuestionMaterial::find_by_qid(db, question_id).await?;
        Entity::find_by_ids(db, material_ids).await
    }

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
            .all(db)
            .await?;

        Ok(materials
            .into_iter()
            .map(|m| m.with_num(&id_sort))
            .sorted_by_key(|m| m.num)
            .collect())
    }

    pub async fn find_by_sim_hash<C>(db: &C, sim_hash: u128) -> anyhow::Result<Vec<Model>>
    where
        C: ConnectionTrait,
    {
        Model::find_by_statement(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            r#"
                SELECT *
                FROM material
                ORDER BY content_sim_hash <~> $1::bit(128)
                LIMIT 10
            "#,
            vec![format!("{sim_hash:0128b}").into()],
        ))
        .all(db)
        .await
        .context("Material::find_by_sim_hash() failed")
    }
}

impl ActiveModel {
    pub async fn insert_on_conflict<C>(mut self, db: &C) -> anyhow::Result<Model>
    where
        C: ConnectionTrait,
    {
        if let Some(content) = self.content.take() {
            // simhash算法去重
            let text_content = {
                Html::parse_fragment(&content)
                    .root_element()
                    .text()
                    .join("")
            };
            let sim_hash = SimHash::<SimSipHasher128, u128, 128>::new(SimSipHasher128::new(1, 2));
            let sim_hash = sim_hash.create_signature(text_content.chars());
            let ms = Entity::find_by_sim_hash(db, sim_hash).await?;
            for m in ms {
                let m_text_content = {
                    Html::parse_fragment(&m.content)
                        .root_element()
                        .text()
                        .join("")
                };
                if m_text_content == text_content {
                    return Ok(m);
                }
                if m_text_content.len() > 100 && text_content.len() > 100 {
                    let edit_distance =
                        textdistance::str::levenshtein(&m_text_content, &text_content);
                    // 95%相似度: 100个字只有5个字不同
                    if edit_distance * 20 < text_content.len().max(m_text_content.len()) {
                        return Ok(m);
                    }
                }
            }

            let extra = serde_json::to_value(&self.extra.take().unwrap_or_default())
                .context("serialize extra failed")?;
            let return_model = if let Some(id) = self.id.take() {
                let sql = r#"
INSERT INTO material (id, content, content_sim_hash, extra)
VALUES ($1, $2, $3::bit(128), $4)
ON CONFLICT (id) DO UPDATE
SET
    content = EXCLUDED.content,
    content_sim_hash = EXCLUDED.content_sim_hash,
    extra = EXCLUDED.extra
RETURNING id, content, extra
"#;
                Model::find_by_statement(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    sql,
                    vec![
                        id.into(),
                        content.clone().into(),
                        format!("{sim_hash:0128b}").into(),
                        extra.into(),
                    ],
                ))
            } else {
                let sql = r#"
INSERT INTO material (content, content_sim_hash, extra)
VALUES ($1, $2::bit(128), $3)
ON CONFLICT (id) DO UPDATE
SET
    content = EXCLUDED.content,
    content_sim_hash = EXCLUDED.content_sim_hash,
    extra = EXCLUDED.extra
    RETURNING id, content, extra
"#;
                Model::find_by_statement(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    sql,
                    vec![
                        content.clone().into(),
                        format!("{sim_hash:0128b}").into(),
                        extra.into(),
                    ],
                ))
            }
            .one(db)
            .await
            .context("insert material failed")?;

            let model = return_model.ok_or_else(|| anyhow!("insert material failed"))?;

            let replaced_content = html::async_replace_img_src(&content, |img_url| {
                let img_url = img_url.to_string();
                Box::pin(async move {
                    let assets = assets::SourceAssets {
                        src_type: SrcType::Material,
                        src_id: model.id,
                        src_url: img_url,
                    }
                    .insert_on_conflict(db)
                    .await?;
                    Ok(assets.compute_storage_url())
                })
            })
            .await?;

            let model = ActiveModel {
                id: Set(model.id),
                content: Set(replaced_content),
                ..Default::default()
            }
            .update(db)
            .await
            .context("update content failed")?;
            Ok(model)
        } else {
            Err(anyhow!("content is required for material insertion"))
        }
    }
}
