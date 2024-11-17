use anyhow::Context;
use dtiku_base::model::schedule_task;
use dtiku_base::model::schedule_task::Progress;
use dtiku_paper::model::exam_category;
use dtiku_paper::model::label;
use dtiku_paper::model::paper;
use dtiku_paper::model::Label;
use futures::StreamExt;
use sea_orm::ConnectionTrait;
use sea_orm::Set;
use serde_json::Value;
use spring::plugin::service::Service;
use spring::tracing;
use spring_sea_orm::DbConn;
use spring_sqlx::sqlx;
use spring_sqlx::ConnectPool;
use sqlx::Row;

#[derive(Clone, Service)]
pub struct FenbiSyncService {
    source_db: ConnectPool,
    target_db: DbConn,
}

impl FenbiSyncService {
    pub async fn start(&self, task: &mut schedule_task::Model) -> anyhow::Result<()> {
        let mut progress = match task.context {
            Value::Null => {
                let total = self
                    .total("select count(*) as total from label where from_ty='fenbi'")
                    .await?;
                let progress = Progress {
                    name: "sync_label".to_string(),
                    total,
                    current: 0,
                };
                *task = task.update_progress(&progress, &self.target_db).await?;
                progress
            }
            _ => serde_json::from_value(task.context.clone())?,
        };
        if progress.name == "sync_label" {
            self.sync_label(task, &mut progress).await?;

            let total = self
                .total("select max(id) as total from paper where from_ty='fenbi'")
                .await?;
            progress = Progress {
                name: "sync_paper".to_string(),
                total,
                current: 0,
            };
            *task = task.update_progress(&progress, &self.target_db).await?;
        }
        if progress.name == "sync_paper" {
            self.sync_paper(task, &mut progress).await?;
        }
        Ok(())
    }

    async fn total(&self, sql: &str) -> anyhow::Result<i64> {
        Ok(sqlx::query(&sql)
            .fetch_one(&self.source_db)
            .await
            .with_context(|| format!("{sql} execute failed"))?
            .try_get("total")
            .context("get total failed")?)
    }

    async fn sync_label(
        &self,
        task: &mut schedule_task::Model,
        progress: &mut Progress<i64>,
    ) -> anyhow::Result<()> {
        let mut stream = sqlx::query_file_as!(OriginLabel, "src/jobs/fenbi/find_label.sql")
            .fetch(&self.source_db);

        while let Some(row) = stream.next().await {
            match row {
                Ok(row) => {
                    let source_id = row.id;
                    let label = row.save_to(&self.target_db).await?;

                    sqlx::query("update label set target_id=? where id=?")
                        .bind(label.id)
                        .bind(source_id)
                        .execute(&self.source_db)
                        .await
                        .context("update source db label target_id failed")?;

                    if progress.increase(1) {
                        *task = task.update_progress(&progress, &self.target_db).await?;
                    }
                }
                Err(e) => tracing::error!("find label failed: {:?}", e),
            };
        }

        Ok(())
    }

    async fn sync_paper(
        &self,
        task: &mut schedule_task::Model,
        progress: &mut Progress<i64>,
    ) -> anyhow::Result<()> {
        while progress.current < progress.total {
            let current = progress.current;
            let mut stream = sqlx::query_file_as!(
                OriginPaper,
                "src/jobs/fenbi/find_paper.sql",
                current,
                current + 1000
            )
            .fetch(&self.source_db);

            while let Some(row) = stream.next().await {
                match row {
                    Ok(row) => {
                        let source_id = row.id;
                        let paper = self.save_paper(row).await?;

                        sqlx::query("update paper set target_id=? where id=?")
                            .bind(paper.id)
                            .bind(source_id)
                            .execute(&self.source_db)
                            .await
                            .context("update source db label target_id failed")?;

                        if progress.increase(1) {
                            *task = task.update_progress(&progress, &self.target_db).await?;
                        }
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

#[derive(Debug, sqlx::FromRow)]
struct OriginLabel {
    exam_root: Option<String>,
    exam_root_prefix: Option<String>,
    exam_name: Option<String>,
    exam_prefix: Option<String>,
    paper_type: Option<String>,
    paper_prefix: Option<String>,
    parent_label: Option<String>,
    label_name: Option<String>,
    id: i64,
}

impl OriginLabel {
    async fn save_to<C: ConnectionTrait>(self, db: &C) -> anyhow::Result<label::Model> {
        let root = exam_category::ActiveModel {
            pid: Set(0),
            name: Set(self.exam_root.expect("exam_root is none")),
            prefix: Set(self.exam_root_prefix.expect("exam_root_prefix is none")),
            ..Default::default()
        }
        .insert_on_conflict(db)
        .await
        .context("insert root exam_category failed")?;

        let second = exam_category::ActiveModel {
            pid: Set(root.id),
            name: Set(self.exam_name.expect("exam_name is none")),
            prefix: Set(self.exam_prefix.expect("exam_prefix is none")),
            ..Default::default()
        }
        .insert_on_conflict(db)
        .await
        .context("insert second exam_category failed")?;

        let leaf = exam_category::ActiveModel {
            pid: Set(second.id),
            name: Set(self.paper_type.expect("paper_type is none")),
            prefix: Set(self.paper_prefix.expect("paper_prefix is none")),
            ..Default::default()
        }
        .insert_on_conflict(db)
        .await
        .context("insert leaf exam_category failed")?;

        let label_name = self.label_name.expect("label_name is none");
        let label = match self.parent_label {
            None => label::ActiveModel {
                name: Set(label_name),
                pid: Set(0),
                exam_id: Set(root.id),
                paper_type: Set(leaf.id),
                ..Default::default()
            },
            Some(parent) => {
                let parent_label =
                    Label::find_by_exam_id_and_paper_type_and_name(db, root.id, leaf.id, &parent)
                        .await?;
                let parent_label = match parent_label {
                    None => label::ActiveModel {
                        name: Set(parent),
                        pid: Set(0),
                        exam_id: Set(root.id),
                        paper_type: Set(leaf.id),
                        ..Default::default()
                    }
                    .insert_on_conflict(db)
                    .await
                    .context("insert parent label failed")?,
                    Some(parent_label) => parent_label,
                };
                label::ActiveModel {
                    name: Set(label_name),
                    pid: Set(parent_label.id),
                    exam_id: Set(root.id),
                    paper_type: Set(leaf.id),
                    ..Default::default()
                }
            }
        };

        let label = label
            .insert_on_conflict(db)
            .await
            .context("insert label failed")?;
        Ok(label)
    }
}

#[derive(Debug, sqlx::FromRow)]
struct OriginPaper {
    name: Option<String>,
    date: Option<String>,
    topic: Option<String>,
    ty: Option<String>,
    chapters: Option<String>,
    id: i64,
}
