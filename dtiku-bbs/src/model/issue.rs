use crate::domain::issue::FullIssue;

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

impl Model {
    pub fn to_full_issue(
        self,
        page_pv: &std::collections::HashMap<String, i32>,
        page_comment: &std::collections::HashMap<String, i32>,
    ) -> FullIssue {
        let key = format!("/bbs/issue/{}", self.id);
        FullIssue {
            id: self.id,
            title: self.title,
            topic: self.topic,
            markdown: self.markdown,
            user_id: self.user_id,
            created: self.created,
            modified: self.modified,
            view: page_pv.get(&key).unwrap_or(&0).to_owned(),
            comment: page_comment.get(&key).unwrap_or(&0).to_owned(),
        }
    }
}
