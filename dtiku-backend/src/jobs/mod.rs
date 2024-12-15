mod fenbi_sync;
mod huatu_sync;
mod offcn_sync;

use crate::plugins::fastembed::TxtEmbedding;
use crate::plugins::jobs::RunningJobs;
use anyhow::Context;
use dtiku_base::model::schedule_task::TaskInstance;
use dtiku_base::model::{enums::ScheduleTaskType, schedule_task};
use fenbi_sync::FenbiSyncService;
use sea_orm::{IntoActiveModel, Set};
use spring::plugin::service::Service;
use spring::{async_trait, plugin::ComponentRegistry, tracing, App};
use spring_sea_orm::DbConn;
use spring_sqlx::ConnectPool;
use spring_stream::handler::TypedConsumer;
use spring_stream::{
    extractor::{Component, Json},
    stream_listener, Consumers,
};

#[stream_listener("task")]
async fn task_schedule(
    Json(mut task): Json<schedule_task::Model>,
    Component(running_jobs): Component<RunningJobs>,
) {
    if running_jobs.is_running(task.ty) {
        return;
    }
    let instance = running_jobs.register_task_if_not_running(task.ty);

    match task.ty {
        ScheduleTaskType::FenbiSync => {
            FenbiSyncService::build(task, instance)
                .expect("build fenbi sync service failed")
                .start()
                .await
        }
    };
}

#[async_trait]
trait JobScheduler {
    async fn start(&mut self) {
        let result = self.inner_start().await;
        if let Err(e) = result {
            tracing::error!("task schedule error:{:?}", e);
            let task = self.current_task();
            schedule_task::ActiveModel {
                id: Set(task.id),
                version: Set(task.version + 1),
                active: Set(false),
                ..Default::default()
            }
            .optimistic_update(&App::global().get_expect_component::<DbConn>())
            .await
            .expect("update error task failed");
        }
    }

    fn current_task(&mut self) -> &mut schedule_task::Model;

    async fn inner_start(&mut self) -> anyhow::Result<()>;
}

pub(crate) fn consumer() -> Consumers {
    Consumers::new().typed_consumer(task_schedule)
}
