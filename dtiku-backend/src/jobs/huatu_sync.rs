use super::{JobScheduler, PaperSyncer};
use crate::plugins::embedding::Embedding;
use anyhow::Context;
use dtiku_base::model::schedule_task::{self, Progress, TaskInstance};
use dtiku_paper::model::{exam_category, label, paper, ExamCategory, FromType, Label};
use futures::StreamExt;
use itertools::Itertools;
use pinyin::ToPinyin;
use sea_orm::ColumnTrait;
use sea_orm::{ActiveValue::Set, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use spring::{async_trait, plugin::service::Service, tracing};
use spring_sea_orm::DbConn;
use spring_sqlx::ConnectPool;
use sqlx::types::Json;

#[derive(Clone, Service)]
#[service(prototype)]
pub struct HuatuSyncService {
    #[inject(component)]
    source_db: ConnectPool,
    #[inject(component)]
    target_db: DbConn,
    #[inject(component)]
    embedding: Embedding,
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

impl HuatuSyncService {
    async fn sync_label(&mut self, progress: &mut Progress<i64>) -> anyhow::Result<()> {
        if progress.total <= 0 {
            return Ok(());
        }
        self.sync_exam_tree().await?;

        // let mut stream = sqlx::query_as::<_, OriginLabel>(
        //     r##"
        //         select
        //             id,
        //             jsonb_extract_path_text(extra,'name') as name,
        //             jsonb_extract_path_text(extra,'parent_name') as parent_name
        //         from "label" l
        //         where from_ty ='huatu'
        // "##,
        // )
        // .fetch(&self.source_db);

        // while let Some(row) = stream.next().await {
        //     match row {
        //         Ok(row) => {
        //             let source_id = row.id;
        //             let label = row.select_from(&self.target_db).await?;

        //             sqlx::query("update label set target_id=$1 where id=$2 and from_ty='huatu'")
        //                 .bind(label.id)
        //                 .bind(source_id)
        //                 .execute(&self.source_db)
        //                 .await
        //                 .context("update source db label target_id failed")?;

        //             if progress.increase(1) {
        //                 self.task = self
        //                     .task
        //                     .update_progress(&progress, &self.target_db)
        //                     .await?;
        //             }
        //         }
        //         Err(e) => tracing::error!("find label failed: {:?}", e),
        //     };
        // }

        Ok(())
    }

    async fn sync_exam_tree(&mut self) -> anyhow::Result<()> {
        if ExamCategory::find()
            .filter(exam_category::Column::FromTy.eq(FromType::Huatu))
            .count(&self.target_db)
            .await?
            > 0
        {
            tracing::info!("exam_category already exists");
            return Ok(());
        }
        let category = sqlx::query_as::<_, OriginExamCategory>(
            r##"
                select extra
                from exam_category_tree
                where from_ty = 'huatu'
                "##,
        )
        .fetch_one(&self.source_db)
        .await
        .context("fetch huatu exam_category_tree failed")?;

        for c in category.extra.0 {
            let root_category = c
                .to_exam_category()
                .insert_on_conflict(&self.target_db)
                .await
                .context("insert exam category failed")?;

            for cc in c.childrens {
                let second_category = cc
                    .to_exam_category_with_pid(root_category.id)
                    .insert_on_conflict(&self.target_db)
                    .await
                    .context("insert exam category failed")?;

                for ccc in cc.childrens {
                    ccc.to_exam_category_with_pid(second_category.id)
                        .insert_on_conflict(&self.target_db)
                        .await
                        .context("insert exam category failed")?;
                }
            }
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
                            coalesce(jsonb_extract_path_text(extra,'area'), jsonb_extract_path_text(extra,'areaName')) as area,
                            coalesce(jsonb_extract_path_text(extra,'name'), jsonb_extract_path_text(extra,'paperName')) as name,
                            coalesce(jsonb_extract_path_text(extra,'type'), '-1') as ty,
                            coalesce(jsonb_extract_path_text(extra,'year'), jsonb_extract_path_text(extra,'paperYear')) as year,
                            coalesce(jsonb_extract_path_text(extra,'modules'), jsonb_extract_path_text(extra,'topicNameList')) as modules,
                            extra
                    from paper p 
                    where from_ty ='huatu' and id > $1 and id <= $2
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

                        sqlx::query("update paper set target_id=$1 where id=$2 and from_ty='huatu")
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

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
pub struct OriginExamCategory {
    pub extra: Json<Vec<ExamTreeNode>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExamTreeNode {
    pub id: i64,
    pub name: String,
    pub childrens: Vec<ExamTreeNode>,
}

impl ExamTreeNode {
    fn to_exam_category(&self) -> exam_category::ActiveModel {
        exam_category::ActiveModel {
            name: Set(self.name.clone()),
            prefix: Set(self
                .name
                .as_str()
                .to_pinyin()
                .into_iter()
                .filter_map(|s| s.map(|py| py.plain()))
                .join("")),
            pid: Set(0),
            from_ty: Set(FromType::Huatu),
            ..Default::default()
        }
    }

    fn to_exam_category_with_pid(&self, pid: i16) -> exam_category::ActiveModel {
        exam_category::ActiveModel {
            name: Set(self.name.clone()),
            prefix: Set(self
                .name
                .as_str()
                .to_pinyin()
                .into_iter()
                .filter_map(|s| s.map(|py| py.plain()))
                .join("")),
            pid: Set(pid),
            from_ty: Set(FromType::Huatu),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct OriginLabel {
    id: Option<i32>,
    name: Option<String>,
    parent_name: Option<String>,
}

impl OriginLabel {
    async fn select_from<C: ConnectionTrait>(self, _db: &C) -> anyhow::Result<label::Model> {
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

enum Modules {
    Blocks(Vec<PaperBlock>),
    Topics(Vec<String>),
}

#[derive(Debug)]
struct PaperBlock {
    name: String,
    qcount: i32,
    category: i32,
    judgeFlag: i32,
}
