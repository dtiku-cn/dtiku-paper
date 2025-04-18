use crate::plugins::fastembed::TxtEmbedding;
use dtiku_base::model::schedule_task::{self, Progress, TaskInstance};
use serde_json::Value;
use spring::{async_trait, plugin::service::Service};
use spring_sea_orm::DbConn;
use spring_sqlx::ConnectPool;

use super::{JobScheduler, PaperSyncer};

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

impl PaperSyncer for HuatuSyncService {}

#[async_trait]
impl JobScheduler for HuatuSyncService {
    fn current_task(&mut self) -> &mut schedule_task::Model {
        &mut self.task
    }

    async fn inner_start(&mut self) -> anyhow::Result<()> {
        let mut progress = match &self.task.context {
            Value::Null => {
                let total = self
                    .total(
                        "select count(*) as total from label where from_ty='huatu'",
                        &self.source_db,
                    )
                    .await?;
                let progress = Progress {
                    name: "sync_label".to_string(),
                    total,
                    current: 0,
                };
                self.task = self
                    .task
                    .update_progress(&progress, &self.target_db)
                    .await?;
                progress
            }
            _ => serde_json::from_value(self.task.context.clone())?,
        };

        if progress.name == "sync_label" {
            self.sync_label(&mut progress).await?;

            let total = self
                .total(
                    "select max(id) as total from paper where from_ty='huatu'",
                    &self.source_db,
                )
                .await?;
            progress = Progress {
                name: "sync_paper".to_string(),
                total,
                current: 0,
            };
            self.task = self
                .task
                .update_progress(&progress, &self.target_db)
                .await?;
        }

        if progress.name == "sync_paper" {
            self.sync_paper(&mut progress).await?;
        }
        Ok(())
    }
}

impl HuatuSyncService{
    async fn sync_label(&mut self, progress: &mut Progress<i64>) -> anyhow::Result<()> {
        

        Ok(())
    }

    async fn sync_paper(&mut self, progress: &mut Progress<i64>) -> anyhow::Result<()> {
        Ok(())
    }
}