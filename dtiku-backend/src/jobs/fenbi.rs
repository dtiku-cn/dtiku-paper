use anyhow::Context;
use dtiku_base::model::schedule_task;
use futures::StreamExt;
use spring::plugin::service::Service;
use spring_sea_orm::DbConn;
use spring_sqlx::sqlx;
use spring_sqlx::ConnectPool;

#[derive(Clone, Service)]
pub struct FenbiSyncService {
    source_db: ConnectPool,
    target_db: DbConn,
}

impl FenbiSyncService {
    pub async fn start(&self, task: schedule_task::Model) {
        self.sync_label().await;
    }

    pub async fn sync_label(&self) -> anyhow::Result<()> {
        let stream = sqlx::query_file_as!(OriginLabel, "src/jobs/fenbi/find_label.sql")
            .fetch(&self.source_db);

        while let Some(row) = stream.next().await {}

        Ok(())
    }
}

#[derive(Debug, sqlx::FromRow)]
struct OriginLabel {
    exam_root: String,
    exam_root_prefix: String,
    exam_name: String,
    exam_prefix: String,
    paper_type: String,
    paper_prefix: String,
    parent_label: String,
    label_name: String,
    id: i64,
}
