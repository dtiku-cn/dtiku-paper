mod fenbi;

use dtiku_base::model::{enums::ScheduleTaskType, schedule_task};
use fenbi::FenbiSyncService;
use spring::App;
use spring_stream::{extractor::Json, stream_listener};

#[stream_listener("task")]
async fn refresh_cache(Json(task): Json<schedule_task::Model>) {
    match task.ty {
        ScheduleTaskType::FenbiSync => {
            App::global()
                .get_component::<FenbiSyncService>()
                .expect("fenbi sync service not found")
                .start(task)
                .await
        }
    }
}
