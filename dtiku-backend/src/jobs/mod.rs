mod fenbi_sync;
mod huatu_sync;
mod idiom_fetch;
mod offcn_sync;
mod web_solution_collect;

use crate::jobs::idiom_fetch::IdiomStatsService;
use crate::jobs::web_solution_collect::WebSolutionCollectService;
use crate::plugins::jobs::RunningJobs;
use anyhow::Context;
use dtiku_base::model::ScheduleTask;
use dtiku_base::model::{enums::ScheduleTaskType, schedule_task};
use fenbi_sync::FenbiSyncService;
use sea_orm::{EntityTrait as _, Set};
use spring::{async_trait, plugin::ComponentRegistry, tracing, App};
use spring_sea_orm::DbConn;
use spring_sqlx::ConnectPool;
use spring_stream::handler::TypedConsumer;
use spring_stream::{
    extractor::{Component, Json},
    stream_listener, Consumers,
};
use sqlx::Row;

#[stream_listener("task")]
// #[axum::debug_handler]
async fn task_schedule(
    Component(running_jobs): Component<RunningJobs>,
    Json(task): Json<schedule_task::Model>,
) {
    let ty = task.ty;
    if running_jobs.is_running(ty) {
        return;
    }
    let instance = running_jobs.register_task_if_not_running(ty);

    match task.ty {
        ScheduleTaskType::FenbiSync => {
            FenbiSyncService::build(task, instance)
                .expect("build fenbi sync service failed")
                .start()
                .await
        }
        ScheduleTaskType::IdiomStats => {
            IdiomStatsService::build(task)
                .expect("build idiom stats service failed")
                .start()
                .await
        }
        ScheduleTaskType::WebSolutionCollect => {
            WebSolutionCollectService::build(task)
                .expect("build web solution service failed")
                .start()
                .await
        }
    };
    running_jobs.remove(&ty);
}

#[async_trait]
trait JobScheduler {
    async fn start(&mut self) {
        let result = self.inner_start().await;
        let success = if let Err(e) = result {
            tracing::error!("task schedule error:{:?}", e);
            let task = self.current_task();
            ScheduleTask::update(schedule_task::ActiveModel {
                id: Set(task.id),
                version: Set(task.version + 1),
                active: Set(false),
                ..Default::default()
            })
            .exec(&App::global().get_expect_component::<DbConn>())
            .await
            .is_err_and(|e| {
                tracing::error!("update task error: {:?}", e);
                false
            })
        } else {
            true
        };
        if success {
            tracing::info!("task schedule success");
        } else {
            tracing::error!("task schedule failed");
        }
    }

    fn current_task(&mut self) -> &mut schedule_task::Model;

    async fn inner_start(&mut self) -> anyhow::Result<()>;
}

trait PaperSyncer {
    /**
     * 查询表的总数量
     */
    async fn total(&self, sql: &str, db: &ConnectPool) -> anyhow::Result<i64> {
        Ok(sqlx::query(&sql)
            .fetch_one(db)
            .await
            .with_context(|| format!("{sql} execute failed"))?
            .try_get("total")
            .context("get total failed")?)
    }
}

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct QuestionIdNumber {
    question_id: i64,
    number: i32,
}

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct MaterialIdNumber {
    material_id: i64,
    number: i32,
}

pub(crate) fn consumer() -> Consumers {
    Consumers::new().typed_consumer(task_schedule)
}
