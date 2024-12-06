mod fenbi_sync;

use dtiku_base::model::{enums::ScheduleTaskType, schedule_task};
use fenbi_sync::FenbiSyncService;
use sea_orm::{IntoActiveModel, Set};
use spring::{plugin::ComponentRegistry, App};
use spring_sea_orm::DbConn;
use spring_stream::{extractor::Json, stream_listener};

// #[stream_listener("task")]
pub async fn refresh_cache(Json(mut task): Json<schedule_task::Model>) {
    let result = match task.ty {
        ScheduleTaskType::FenbiSync => {
            App::global()
                .get_component::<FenbiSyncService>()
                .expect("fenbi sync service not found")
                .start(&mut task)
                .await
        }
    };

    if let Err(e) = result {
        let db = App::global()
            .get_component::<DbConn>()
            .expect("fenbi sync service not found");
        let error_count = task.error_count;
        let mut active_model = task.into_active_model();
        active_model.active = Set(false);
        active_model.error_cause = Set(format!("{:?}", e));
        active_model.error_count = Set(error_count + 1);
        active_model
            .optimistic_update(&db)
            .await
            .expect("update error task failed");
    }
}
