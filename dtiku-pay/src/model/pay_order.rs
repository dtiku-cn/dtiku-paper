use super::OrderStatus;
pub use super::_entities::pay_order::*;
use anyhow::Context;
use sea_orm::{
    prelude::DateTime, sqlx::types::chrono::Local, ActiveModelBehavior, ActiveValue::Set,
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, FromQueryResult, QueryFilter, QuerySelect, Statement,
};
use serde::Serialize;
use spring::{async_trait, plugin::ComponentRegistry, App};
use spring_stream::Producer;

#[derive(Debug, FromQueryResult, Serialize)]
pub struct PayStatsByDay {
    pub day: DateTime,
    pub paid_count: i64,
    pub paid_amount: i64,
    pub unpaid_user_count: i64,
}

#[derive(Debug, FromQueryResult, Serialize)]
pub struct UnpaidUserCount {
    pub unpaid_user_count: i64,
}

#[async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(mut self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if insert {
            self.created = Set(Local::now().naive_local());
            self.status = Set(OrderStatus::Created);
        }
        self.modified = Set(Local::now().naive_local());
        Ok(self)
    }

    async fn after_save<C>(model: Model, _db: &C, insert: bool) -> Result<Model, DbErr>
    where
        C: ConnectionTrait,
    {
        if insert {
            let producer = App::global().get_expect_component::<Producer>();
            let _ = producer.send_json("pay_order", &model).await;
        } else if model.confirm.is_some() {
            let producer = App::global().get_expect_component::<Producer>();
            let _ = producer.send_json("pay_order.confirm", &model).await;
        }
        Ok(model)
    }
}

impl Entity {
    pub async fn find_wait_confirm_after<C: ConnectionTrait>(
        db: &C,
        time: DateTime,
    ) -> anyhow::Result<Vec<Model>> {
        Entity::find()
            .filter(Column::Confirm.is_null().and(Column::Created.gt(time)))
            .all(db)
            .await
            .with_context(|| format!("find_wait_confirm({time:?}) failed"))
    }

    pub async fn find_order_status<C: ConnectionTrait>(
        db: &C,
        order_id: i32,
        user_id: i32,
    ) -> anyhow::Result<Option<OrderStatus>> {
        Entity::find()
            .select_only()
            .column(Column::Status)
            .filter(Column::Id.eq(order_id).and(Column::UserId.eq(user_id)))
            .into_tuple()
            .one(db)
            .await
            .with_context(|| format!("find_order_status({order_id},{user_id}) failed"))
    }

    pub async fn stats_by_day<C: ConnectionTrait>(db: &C) -> anyhow::Result<Vec<PayStatsByDay>> {
        let db_backend = db.get_database_backend();

        let stmt = Statement::from_sql_and_values(
            db_backend,
            r#"
            WITH date_range AS (
                SELECT 
                    COALESCE(
                        LEAST(
                            MIN(date_trunc('day', confirm)),
                            MIN(date_trunc('day', created))
                        ),
                        CURRENT_DATE - INTERVAL '30 days'
                    ) as min_date,
                    CURRENT_DATE as max_date
                FROM pay_order
            ),
            date_series AS (
                SELECT generate_series(
                    (SELECT min_date FROM date_range),
                    (SELECT max_date FROM date_range),
                    '1 day'::interval
                )::timestamp as day
            ),
            paid_stats AS (
                SELECT 
                    date_trunc('day', confirm) as day,
                    COUNT(*) as paid_count,
                    SUM(CASE 
                        WHEN level = 'monthly' THEN 1000
                        WHEN level = 'quarterly' THEN 2500
                        WHEN level = 'half_year' THEN 4000
                        WHEN level = 'annual' THEN 6000
                        ELSE 0
                    END) as paid_amount
                FROM pay_order
                WHERE status = 'paid' AND confirm IS NOT NULL
                GROUP BY day
            ),
            unpaid_stats AS (
                SELECT 
                    date_trunc('day', created) as day,
                    COUNT(DISTINCT user_id) as unpaid_user_count
                FROM pay_order
                WHERE status = 'created'
                GROUP BY day
            )
            SELECT 
                date_series.day,
                COALESCE(paid_stats.paid_count, 0) as paid_count,
                COALESCE(paid_stats.paid_amount, 0) as paid_amount,
                COALESCE(unpaid_stats.unpaid_user_count, 0) as unpaid_user_count
            FROM date_series
            LEFT JOIN paid_stats ON date_series.day = paid_stats.day
            LEFT JOIN unpaid_stats ON date_series.day = unpaid_stats.day
            ORDER BY date_series.day
            "#
            .to_owned(),
            vec![],
        );

        PayStatsByDay::find_by_statement(stmt)
            .all(db)
            .await
            .context("PayStatsByDay execute failed")
    }

    pub async fn total_unpaid_user_count<C: ConnectionTrait>(db: &C) -> anyhow::Result<i64> {
        let db_backend = db.get_database_backend();

        let stmt = Statement::from_sql_and_values(
            db_backend,
            r#"
            SELECT COUNT(DISTINCT user_id) as unpaid_user_count
            FROM pay_order
            WHERE status = 'created'
            "#
            .to_owned(),
            vec![],
        );

        let result = UnpaidUserCount::find_by_statement(stmt)
            .one(db)
            .await
            .context("UnpaidUserCount execute failed")?;

        Ok(result.map(|r| r.unpaid_user_count).unwrap_or(0))
    }
}
