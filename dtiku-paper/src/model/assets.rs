pub use super::_entities::assets::*;
use anyhow::Context;
use sea_orm::{
    sea_query::OnConflict, sqlx::types::chrono::Local, ActiveModelBehavior, ActiveValue::Set,
    ConnectionTrait, DbErr, EntityTrait as _,
};
use spring::async_trait;

#[async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(mut self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if insert {
            self.created = Set(Local::now().naive_local());
            if let Some(src_url) = self.src_url.take() {
                if !self.src_hash.is_set() {
                    self.src_hash = Set(md5::compute(&src_url).0.to_vec());
                }
                self.src_url = Set(src_url);
            }
        }
        self.modified = Set(Local::now().naive_local());
        Ok(self)
    }
}

impl Model {
    pub fn compute_storage_url(&self) -> String {
        let date = self.created.format("%Y/%m/%d").to_string();
        let src_type = &self.src_type;
        let id = self.id;
        format!("//s.dtiku.cn/{src_type}/{date}/{id}")
    }
}

impl ActiveModel {
    pub async fn insert_on_conflict<C>(mut self, db: &C) -> anyhow::Result<Model>
    where
        C: ConnectionTrait,
    {
        self = self
            .before_save(db, true)
            .await
            .context("before insert assets failed")?;
        Entity::insert(self)
            .on_conflict(
                OnConflict::columns([Column::SrcType, Column::SrcId, Column::SrcHash])
                    .update_columns([Column::SrcUrl, Column::Modified])
                    .to_owned(),
            )
            .exec_with_returning(db)
            .await
            .context("insert assets failed")
    }
}
