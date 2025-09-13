use crate::plugins::embedding::Embedding;
use anyhow::Context;
use dtiku_base::model::schedule_task::{self, Progress, TaskInstance};
use dtiku_paper::model::{label, paper};
use futures::StreamExt as _;
use sea_orm::ConnectionTrait;
use serde_json::Value;
use spring::{async_trait, plugin::service::Service, tracing};
use spring_sea_orm::DbConn;
use spring_sqlx::ConnectPool;

use super::{JobScheduler, PaperSyncer};

#[derive(Clone, Service)]
#[service(prototype)]
pub struct OffcnSyncService {
    #[inject(component)]
    source_db: ConnectPool,
    #[inject(component)]
    target_db: DbConn,
    #[inject(component)]
    embedding: Embedding,
    task: schedule_task::Model,
    instance: TaskInstance,
}

impl PaperSyncer for OffcnSyncService {}

#[async_trait]
impl JobScheduler for OffcnSyncService {
    fn current_task(&mut self) -> &mut schedule_task::Model {
        &mut self.task
    }

    async fn inner_start(&mut self) -> anyhow::Result<()> {
        let mut progress = match &self.task.context {
            Value::Null => {
                let total = self
                    .total(
                        "select count(*) as total from label where from_ty='offcn'",
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
                    "select max(id) as total from paper where from_ty='offcn'",
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

impl OffcnSyncService {
    async fn sync_label(&mut self, progress: &mut Progress<i64>) -> anyhow::Result<()> {
        if progress.total <= 0 {
            return Ok(());
        }
        let mut stream = sqlx::query_as::<_, OriginLabel>(
            r##"
            select 
                id,
                jsonb_extract_path_text(extra,'name') as name,
                jsonb_extract_path(extra,'type') as ty,
                jsonb_extract_path(extra,'parent_id') as parent_id,
                jsonb_extract_path_text(extra,'parent_name') as parent_name
            from "label" l 
            where from_ty ='offcn'
        "##,
        )
        .fetch(&self.source_db);

        while let Some(row) = stream.next().await {
            match row {
                Ok(row) => {
                    let source_id = row.id;
                    let label = row.save_to(&self.target_db).await?;

                    sqlx::query("update label set target_id=$1 where id=$2 and from_ty='offcn'")
                        .bind(label.id)
                        .bind(source_id)
                        .execute(&self.source_db)
                        .await
                        .context("update source db label target_id failed")?;

                    if progress.increase(1) {
                        self.task = self
                            .task
                            .update_progress(&progress, &self.target_db)
                            .await?;
                    }
                }
                Err(e) => tracing::error!("find label failed: {:?}", e),
            };
        }

        Ok(())
    }

    async fn sync_paper(&mut self, progress: &mut Progress<i64>) -> anyhow::Result<()> {
        while progress.current < progress.total {
            let current = progress.current;
            let next_step_id: i64 = current + 1000;
            let mut stream = sqlx::query_as::<_, OriginPaper>(
                r##"
                    select
                        id,
                        label_id,
                        jsonb_extract_path_text(extra,'list') as list,
                        jsonb_extract_path_text(extra,'title') as title,
                        jsonb_extract_path_text(extra,'content') as content,
                        jsonb_extract_path_text(extra,'paper_pattern') as paper_pattern,
                        extra
                    from paper p
                    where  from_ty ='offcn' and id > $1 and id <= $2
                    "##,
            )
            .bind(current)
            .bind(next_step_id)
            .fetch(&self.source_db);

            while let Some(row) = stream.next().await {
                match row {
                    Ok(row) => {
                        let source_id = row.id;
                        let paper = self.save_paper(row).await?;

                        sqlx::query("update paper set target_id=$1 where id=$2 and from_ty='fenbi")
                            .bind(paper.id)
                            .bind(source_id)
                            .execute(&self.source_db)
                            .await
                            .context("update source db label target_id failed")?;

                        progress.current = source_id;
                        self.task = self
                            .task
                            .update_progress(&progress, &self.target_db)
                            .await?;
                    }
                    Err(e) => tracing::error!("find label failed: {:?}", e),
                };
            }
        }
        Ok(())
    }

    async fn save_paper(&self, paper: OriginPaper) -> anyhow::Result<paper::Model> {
        todo!()
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct OriginLabel {
    id: Option<i32>,
    name: Option<String>,
    ty: Option<i32>,
    parent_id: Option<i32>,
    parent_name: Option<String>,
}

impl OriginLabel {
    async fn save_to<C: ConnectionTrait>(self, db: &C) -> anyhow::Result<label::Model> {
        todo!()
    }
}

#[derive(Debug, sqlx::FromRow)]
struct OriginPaper {
    name: Option<String>,
    date: Option<String>,
    topic: Option<String>,
    ty: Option<i32>,
    chapters: Option<String>,
    id: i64,
    label_id: i64,
}

impl OriginPaper {
    async fn save_paper<C: ConnectionTrait>(
        self,
        db: &C,
        label_id: i32,
    ) -> anyhow::Result<paper::Model> {
        todo!()
    }
}

struct PaperBlock {
    name: String,
    blockId: i32,
    doneCount: i32,
    totalCount: i32,
}
