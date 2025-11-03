pub use super::_entities::user_info::*;
use crate::query::UserQuery;
use anyhow::Context;
use chrono::Days;
use sea_orm::entity::prelude::*;
use sea_orm::{
    sqlx::types::chrono::Local, ActiveModelBehavior, ActiveValue::Set, ColumnTrait,
    ConnectionTrait, DbErr, EntityTrait, FromQueryResult, QueryFilter, Statement,
};
use serde::Serialize;
use spring::plugin::ComponentRegistry as _;
use spring::{async_trait, App};
use spring_redis::redis::AsyncCommands;
use spring_redis::{cache, Redis};
use spring_sea_orm::pagination::{Page, Pagination, PaginationExt};

#[derive(Debug, FromQueryResult, Serialize)]
pub struct UserStatsByDay {
    pub day: DateTime,
    pub count: i64,
}

impl Model {
    pub fn is_expired(&self) -> bool {
        self.expired < Local::now().naive_local()
    }

    pub fn due_time(&self) -> String {
        let now = Local::now().naive_local();
        let diff = self.expired - now;

        if diff.num_days() >= 0 && diff.num_days() <= 7 {
            format!("还有{}天", diff.num_days())
        } else {
            self.expired.format("%Y-%m-%d %H:%M").to_string()
        }
    }
}

#[async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(mut self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        let now = Local::now().naive_local();
        if insert {
            self.created = Set(now);
            let expired = now
                .checked_add_days(Days::new(7))
                .expect("days add overflow");
            self.expired = Set(expired);
        }
        self.modified = Set(now);
        Ok(self)
    }

    async fn after_save<C>(model: Model, _db: &C, insert: bool) -> Result<Model, DbErr>
    where
        C: ConnectionTrait,
    {
        if !insert {
            let user_id = model.id;
            let mut redis = App::global().get_expect_component::<Redis>();
            let _: () = redis.del(format!("user:{user_id}")).await.unwrap();
        }
        Ok(model)
    }
}

impl Entity {
    #[cache("user:{id}", expire = 86400, unless = result.is_none())]
    pub async fn find_user_by_id<C: ConnectionTrait>(
        db: &C,
        id: i32,
    ) -> anyhow::Result<Option<Model>> {
        Entity::find_by_id(id)
            .one(db)
            .await
            .with_context(|| format!("UserInfo::find_user_by_id({id}) failed"))
    }

    pub async fn find_user_by_ids<C: ConnectionTrait>(
        db: &C,
        ids: Vec<i32>,
    ) -> anyhow::Result<Vec<Model>> {
        Entity::find()
            .filter(Column::Id.is_in(ids))
            .all(db)
            .await
            .context("UserInfo::find_user_by_ids() failed")
    }

    pub async fn find_page_by_query<C: ConnectionTrait>(
        db: &C,
        query: UserQuery,
        pagination: &Pagination,
    ) -> anyhow::Result<Page<Model>> {
        Entity::find()
            .filter(query)
            .page(db, &pagination)
            .await
            .context("UserInfo::find_page_by_query() failed")
    }

    pub async fn stats_by_day<C: ConnectionTrait>(db: &C) -> anyhow::Result<Vec<UserStatsByDay>> {
        let db_backend = db.get_database_backend();

        let stmt = Statement::from_sql_and_values(
            db_backend,
            r#"
            SELECT date_trunc('day', created) as day, COUNT(*) as count
            FROM user_info
            GROUP BY day
            ORDER BY day
            "#
            .to_owned(),
            vec![],
        );

        UserStatsByDay::find_by_statement(stmt)
            .all(db)
            .await
            .context("UserStatsByDay execute failed")
    }
}
