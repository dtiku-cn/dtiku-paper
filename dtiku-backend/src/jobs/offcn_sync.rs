use super::{JobScheduler, PaperSyncer};
use crate::{
    jobs::{MaterialIdNumber, QuestionIdNumber},
    plugins::embedding::Embedding,
};
use anyhow::Context;
use dtiku_base::model::schedule_task::{self, Progress, TaskInstance};
use dtiku_paper::model::{label, material, paper, paper_material, question, solution, FromType};
use futures::StreamExt as _;
use itertools::Itertools as _;
use sea_orm::{ActiveValue::Set, ConnectionTrait};
use serde::Deserialize;
use serde_json::Value;
use spring::{async_trait, plugin::service::Service, tracing};
use spring_sea_orm::DbConn;
use spring_sqlx::ConnectPool;
use sqlx::{types::Json, Row};
use std::{collections::HashMap, num::ParseIntError};

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
        let source_paper_id = paper.id;
        let target_exam_id: i32 =
            sqlx::query("select target_id from label where id = $1 and from_ty='offcn'")
                .bind(paper.label_id)
                .fetch_one(&self.source_db)
                .await
                .with_context(|| format!("find target_id for label#{}", paper.label_id))?
                .try_get("target_id")
                .context("get target_id failed")?;
        let paper = paper.save_paper(&self.target_db, target_exam_id).await?;

        self.sync_questions_and_materials(source_paper_id, &paper)
            .await?;

        Ok(paper)
    }

    async fn sync_questions_and_materials(
        &self,
        source_paper_id: i64,
        paper: &paper::Model,
    ) -> anyhow::Result<()> {
        let question_ids: Vec<QuestionIdNumber> = sqlx::query_as(
            r##"
            select
                question_id,
                number
            from paper_question
            where from_ty = 'offcn'
            and paper_id = $1
            order by number
            "##,
        )
        .bind(source_paper_id)
        .fetch_all(&self.source_db)
        .await
        .with_context(|| format!("find question_ids({source_paper_id}) failed"))?;

        let qid_num_map: HashMap<_, _> = question_ids
            .into_iter()
            .map(|q| (q.question_id, q.number))
            .collect();
        let qids = qid_num_map.keys().cloned().collect_vec();

        let material_ids: Vec<MaterialIdNumber> = sqlx::query_as(
            r##"
            select
                material_id,
                number
            from paper_material
            where from_ty = 'offcn'
            and paper_id = $1
            order by number
            "##,
        )
        .bind(source_paper_id)
        .fetch_all(&self.source_db)
        .await
        .with_context(|| format!("find material_ids({source_paper_id}) failed"))?;

        let mid_num_map: HashMap<_, _> = material_ids
            .into_iter()
            .map(|m| (m.material_id, m.number + 1))
            .collect();
        let mut mids = mid_num_map.keys().cloned().collect_vec();

        let questions = sqlx::query_as::<_, OriginQuestion>(
            r##"
            select
                id,
                extra->>'type' as ty,
                extra->>'stem' as content,
                extra->>'choices' as choices,
                extra->>'answer' as answer,
                extra->>'explain_a' as explain,
                extra->>'analysis' as analysis,
                extra->>'step_explanation' as step_explanation,
                extra->>'multi_material_id' as multi_material_id
            from question
            where from_ty='offcn'
            and id = any($1)
        "##,
        )
        .bind(qids)
        .fetch_all(&self.source_db)
        .await
        .with_context(|| format!("find questions({source_paper_id}) failed"))?;

        for q in &questions {
            if let Some(multi_material_id) = &q.multi_material_id {
                mids.extend(
                    multi_material_id
                        .split(",")
                        .map(|mid| mid.parse())
                        .collect::<Result<Vec<i64>, ParseIntError>>()
                        .context("parse i32 failed")?,
                );
            }
        }

        let materials = sqlx::query_as::<_, OriginMaterial>(
            r##"
            select
                id,
                target_id,
                jsonb_extract_path_text(extra,'content') as content
            from material
            where from_ty = 'offcn'
            and id = any($1)
        "##,
        )
        .bind(mids)
        .fetch_all(&self.source_db)
        .await
        .with_context(|| format!("find material({source_paper_id}) failed"))?;

        self.save_questions_and_materials(questions, materials, paper, &qid_num_map, &mid_num_map)
            .await?;
        Ok(())
    }

    async fn save_questions_and_materials(
        &self,
        questions: Vec<OriginQuestion>,
        materials: Vec<OriginMaterial>,
        paper: &paper::Model,
        qid_num_map: &HashMap<i64, i32>,
        mid_num_map: &HashMap<i64, i32>,
    ) -> anyhow::Result<()> {
        let mut material_num = 1;
        for m in materials {
            let num = mid_num_map
                .get(&m.id)
                .expect("mid is not exists in mid_num_map");
            material_num = material_num.max(*num);
            self.save_material(m, paper.id, *num).await?;
        }

        for q in questions {
            let correct_ratio = 1.0;
            let num = qid_num_map
                .get(&q.id)
                .expect("qid is not exists in qid_num_map");
            let mut question = q.to_question(&self.embedding).await?;
            question.exam_id = Set(paper.exam_id);
            question.paper_type = Set(paper.paper_type);
            let q_in_db = question
                .insert_on_conflict(&self.target_db)
                .await
                .context("insert question failed")?;
            let mut solution = q.to_solution()?;
            solution.from_ty = Set(FromType::Offcn);
            solution.question_id = Set(q_in_db.id);
            solution.insert_on_conflict(&self.target_db).await?;

            // todo save paper_question
            
        }

        Ok(())
    }

    async fn save_material(
        &self,
        m: OriginMaterial,
        paper_id: i32,
        num: i32,
    ) -> Result<(), anyhow::Error> {
        let source_material_id = m.id;
        let material = TryInto::<material::ActiveModel>::try_into(m)?;
        let m_in_db = material.insert_on_conflict(&self.target_db).await?;
        paper_material::ActiveModel {
            paper_id: Set(paper_id),
            material_id: Set(m_in_db.id),
            sort: Set(num as i16),
        }
        .insert_on_conflict(&self.target_db)
        .await
        .context("insert paper_material failed")?;
        sqlx::query("update material set target_id=$1 where id=$2 and from_ty='offcn'")
            .bind(m_in_db.id)
            .bind(source_material_id)
            .execute(&self.source_db)
            .await
            .context("update source db paper target_id failed")?;
        Ok(())
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

#[derive(Debug, Clone, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct OriginMaterial {
    pub id: i64,
    pub target_id: Option<i32>,
    pub content: String,
}

impl TryInto<material::ActiveModel> for OriginMaterial {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<material::ActiveModel, Self::Error> {
        let mut am = material::ActiveModel {
            content: Set(self.content),
            extra: Set(vec![]),
            ..Default::default()
        };
        if let Some(id) = self.target_id {
            am.id = Set(id);
        }
        Ok(am)
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct OriginQuestion {
    id: i64,
    ty: i16,
    content: String,
    choices: Option<Json<Vec<Choice>>>,
    answer: Option<Json<Vec<String>>>,
    explain: Option<String>,
    analysis: Option<String>,
    step_explanation: Option<Json<Vec<String>>>,
    multi_material_id: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
pub struct Choice {
    pub choice: String,
    pub choice_id: i64,
    pub is_correct: i64,
    pub question_id: i64,
}

impl OriginQuestion {
    async fn to_question(&self, embedding: &Embedding) -> anyhow::Result<question::ActiveModel> {
        todo!()
    }

    fn to_solution(&self) -> anyhow::Result<solution::ActiveModel> {
        todo!()
    }
}
