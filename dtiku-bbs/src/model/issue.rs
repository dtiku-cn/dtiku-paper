use super::IssueQuery;
pub use super::_entities::issue::*;
use anyhow::Context;
use sea_orm::{
    sea_query::IntoCondition, sqlx::types::chrono::Local, ActiveModelBehavior, ConnectionTrait,
    DbErr, EntityTrait, QueryFilter, Set,
};
use spring::async_trait;
use spring_sea_orm::pagination::{Page, Pagination, PaginationExt};

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

impl Entity {
    pub async fn find_issue_by_id<C: ConnectionTrait>(
        db: &C,
        id: i32,
    ) -> anyhow::Result<Option<Model>> {
        Entity::find_by_id(id)
            .one(db)
            .await
            .with_context(|| format!("Issue::find_by_id({id}) failed"))
    }

    pub async fn search<C: ConnectionTrait>(
        db: &C,
        query: &IssueQuery,
        pagination: &Pagination,
    ) -> anyhow::Result<Page<Model>> {
        Entity::find()
            .filter(query.clone().into_condition())
            .page(db, &pagination)
            .await
            .context("find issue failed")
    }
}
