use crate::plugins::fastembed::TxtEmbedding;
use dtiku_base::model::schedule_task::{self, TaskInstance};
use spring::{async_trait, plugin::service::Service};
use spring_sea_orm::DbConn;
use spring_sqlx::ConnectPool;

use super::JobScheduler;

#[derive(Clone, Service)]
#[prototype]
pub struct HuatuSyncService {
    #[inject(component)]
    source_db: ConnectPool,
    #[inject(component)]
    target_db: DbConn,
    #[inject(component)]
    txt_embedding: TxtEmbedding,
    task: schedule_task::Model,
    instance: TaskInstance,
}

#[async_trait]
impl JobScheduler for HuatuSyncService {
    fn current_task(&mut self) -> &mut schedule_task::Model {
        &mut self.task
    }

    async fn inner_start(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}
