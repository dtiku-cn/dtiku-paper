mod fenbi_sync;
mod huatu_sync;
mod offcn_sync;

use crate::plugins::jobs::RunningJobs;
use dtiku_base::model::{enums::ScheduleTaskType, schedule_task};
use fenbi_sync::FenbiSyncService;
use sea_orm::{IntoActiveModel, Set};
use spring::{plugin::ComponentRegistry, tracing, App};
use spring_sea_orm::DbConn;
use spring_stream::{
    extractor::{Component, Json},
    stream_listener,
};

#[stream_listener("task")]
pub async fn refresh_cache(
    Json(mut task): Json<schedule_task::Model>,
    Component(running_jobs): Component<RunningJobs>,
) {
    if running_jobs.is_running(task.ty) {
        return;
    }
    running_jobs.register_task_if_not_running(task.ty);
    
    let app = App::global();

    let result = match task.ty {
        ScheduleTaskType::FenbiSync => {
            app.get_component::<FenbiSyncService>()
                .expect("fenbi sync service not found")
                .start(&mut task)
                .await
        }
    };

    if let Err(e) = result {
        tracing::error!("task schedule error:{:?}", e);
        let db = app
            .get_component::<DbConn>()
            .expect("fenbi sync service not found");
        schedule_task::ActiveModel {
            id: Set(task.id),
            version: Set(task.version + 1),
            active: Set(false),
            ..Default::default()
        }
        .optimistic_update(&db)
        .await
        .expect("update error task failed");
    }
}
