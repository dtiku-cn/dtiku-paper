pub use super::_entities::idiom::*;
use anyhow::Context as _;
use sea_orm::{
    sea_query::OnConflict, sqlx::types::chrono::Local, ActiveModelBehavior, ActiveValue::Set,
    ColumnTrait, ConnectionTrait, DbErr, DerivePartialModel, EntityTrait as _, FromJsonQueryResult,
    FromQueryResult, QueryFilter, QuerySelect as _,
};
use serde::{Deserialize, Serialize};
use spring::async_trait;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct IdiomExplain {
    pub shiyidetail: String,
    pub liju: String,
    pub jyc: Vec<String>,
    pub fyc: Vec<String>,
}

#[derive(Clone, Debug, DerivePartialModel, FromQueryResult)]
#[sea_orm(entity = "Entity")]
pub struct BriefIdiom {
    #[sea_orm(from_col = "id")]
    pub id: i32,
    #[sea_orm(from_col = "text")]
    pub text: String,
    #[sea_orm(from_col = "explain")]
    pub explain: String,
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
