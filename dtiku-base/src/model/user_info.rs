pub use super::_entities::user_info::*;
use crate::query::UserQuery;
use anyhow::Context;
use chrono::{Days, Duration, NaiveDate};
use sea_orm::entity::prelude::*;
use sea_orm::QueryOrder;
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

impl ActiveModel {
    pub async fn add_expiration_days<C: ConnectionTrait>(
        db: &C,
        user_id: i32,
        days: i64,
    ) -> anyhow::Result<Model> {
        // 先查询用户当前的过期时间
        let user = Entity::find_by_id(user_id)
            .one(db)
            .await
            .with_context(|| format!("查询用户失败"))?
            .with_context(|| format!("用户不存在"))?;
        
        let now = Local::now().naive_local();
        // 如果用户已过期，从当前时间开始计算；否则在原过期时间基础上延长
        let base_time = if user.expired < now {
            now
        } else {
            user.expired
        };
        let expires = base_time + Duration::days(days);
        
        Self {
            id: Set(user_id),
            expired: Set(expires),
            ..Default::default()
        }
        .update(db)
        .await
        .with_context(|| format!("update user failed"))
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
            .order_by_desc(Column::Created)
            .page(db, &pagination)
            .await
            .context("UserInfo::find_page_by_query() failed")
    }

    pub async fn stats_by_day<C: ConnectionTrait>(
        db: &C,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> anyhow::Result<Vec<UserStatsByDay>> {
        let db_backend = db.get_database_backend();

        // 默认最近30天
        let end = end_date.unwrap_or_else(|| Local::now().date_naive());
        let start = start_date.unwrap_or_else(|| end - chrono::Duration::days(30));

        let stmt = Statement::from_sql_and_values(
            db_backend,
            r#"
            WITH date_series AS (
                SELECT generate_series(
                    $1::date,
                    $2::date,
                    '1 day'::interval
                )::timestamp as day
            ),
            user_stats AS (
                SELECT date_trunc('day', created) as day, COUNT(*) as count
                FROM user_info
                WHERE date_trunc('day', created) >= $1::date 
                  AND date_trunc('day', created) <= $2::date
                GROUP BY day
            )
            SELECT 
                date_series.day,
                COALESCE(user_stats.count, 0) as count
            FROM date_series
            LEFT JOIN user_stats ON date_series.day = user_stats.day
            ORDER BY date_series.day
            "#
            .to_owned(),
            vec![start.into(), end.into()],
        );

        UserStatsByDay::find_by_statement(stmt)
            .all(db)
            .await
            .context("UserStatsByDay execute failed")
    }
}
