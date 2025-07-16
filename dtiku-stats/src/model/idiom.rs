pub use super::_entities::idiom::*;
use sea_orm::{
    sea_query::OnConflict, sqlx::types::chrono::Local, ActiveModelBehavior, ActiveModelTrait,
    ActiveValue::Set, ConnectionTrait, DbErr, EntityTrait as _, FromJsonQueryResult,
};
use serde::{Deserialize, Serialize};
use spring::async_trait;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct IdiomExplain {
    pub shiyi: String,
    pub shiyidetail: String,
    pub liju: String,
    pub jyc: Vec<String>,
    pub fyc: Vec<String>,
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
