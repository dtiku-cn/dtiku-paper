use super::IssueQuery;
pub use super::_entities::issue::*;
use crate::model::TopicType;
use anyhow::Context;
use sea_orm::{
    prelude::DateTime, sea_query::IntoCondition, sqlx::types::chrono::Local, ActiveModelBehavior,
    ColumnTrait, ConnectionTrait, DbErr, DerivePartialModel, EntityTrait, FromQueryResult,
    QueryFilter, QueryOrder, QuerySelect, Set,
};
use spring::async_trait;
use spring_sea_orm::pagination::{Page, Pagination, PaginationExt};

#[derive(Clone, Debug, DerivePartialModel, FromQueryResult)]
#[sea_orm(entity = "Entity")]
pub struct ListIssue {
    #[sea_orm(from_col = "id")]
    pub id: i32,
    #[sea_orm(from_col = "topic")]
    pub topic: TopicType,
    #[sea_orm(from_col = "title")]
    pub title: String,
    #[sea_orm(from_col = "pin")]
    pub pin: bool,
    #[sea_orm(from_col = "user_id")]
    pub user_id: i32,
    #[sea_orm(from_col = "created")]
    pub created: DateTime,
    #[sea_orm(from_col = "modified")]
    pub modified: DateTime,
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

    pub async fn find_pins_by_topic<C: ConnectionTrait>(
        db: &C,
        topic: Option<TopicType>,
    ) -> anyhow::Result<Vec<ListIssue>> {
        let mut filter = Column::Pin.eq(true);
        if let Some(topic) = topic {
            filter = filter.eq(Column::Topic.eq(topic));
        }
        Entity::find()
            .filter(filter)
            .order_by_desc(Column::Created)
            .limit(3)
            .into_partial_model::<ListIssue>()
            .all(db)
            .await
            .context("find issue failed")
    }

    pub async fn search<C: ConnectionTrait>(
        db: &C,
        query: &IssueQuery,
        pagination: &Pagination,
    ) -> anyhow::Result<Page<ListIssue>> {
        Entity::find()
            .filter(query.clone().into_condition())
            .order_by_desc(Column::Created)
            .into_partial_model::<ListIssue>()
            .page(db, &pagination)
            .await
            .context("find issue failed")
    }
}
