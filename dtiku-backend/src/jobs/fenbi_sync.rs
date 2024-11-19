use crate::utils::regex::pick_year;
use anyhow::Context;
use dtiku_base::model::schedule_task;
use dtiku_base::model::schedule_task::Progress;
use dtiku_paper::model::exam_category;
use dtiku_paper::model::label;
use dtiku_paper::model::paper;
use dtiku_paper::model::paper::Chapters;
use dtiku_paper::model::paper::EssayCluster;
use dtiku_paper::model::paper::PaperBlock;
use dtiku_paper::model::paper::PaperChapter;
use dtiku_paper::model::Label;
use futures::StreamExt;
use itertools::Itertools;
use sea_orm::ConnectionTrait;
use sea_orm::Set;
use serde::Deserialize;
use serde::Serialize;
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
        let mut stream = sqlx::query_as::<_,OriginLabel>(r##"
        select 
            jsonb_extract_path_text(extra,'course_set','liveConfigItem','name') as exam_root,
            jsonb_extract_path_text(extra,'course_set','liveConfigItem','prefix') as exam_root_prefix,
            jsonb_extract_path_text(extra,'course_set','courseSet','name') as exam_name,
            jsonb_extract_path_text(extra,'course_set','courseSet','prefix') as exam_prefix,
            jsonb_extract_path_text(extra,'course','name') as paper_type,
            jsonb_extract_path_text(extra,'course','prefix') as paper_prefix,
            jsonb_extract_path_text(extra,'parent','name') as parent_label,
            jsonb_extract_path_text(extra,'name') as label_name,
            id
        from label
        where from_ty = 'fenbi'
        order by exam_root,exam_name,paper_type,parent_label,label_name
        "##).fetch(&self.source_db);

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
            let next_step_id: i64 = current + 1000;
            let mut stream = sqlx::query_as::<_, OriginPaper>(
                r##"
                    select 
                        jsonb_extract_path_text(extra,'name') as name,
                        jsonb_extract_path_text(extra,'date') as date,
                        jsonb_extract_path_text(extra,'topic') as topic,
                        jsonb_extract_path_text(extra,'type') as ty,
                        jsonb_extract_path_text(extra,'chapters') as chapters,
                        id,
                        label_id
                    from paper
                    where from_ty = 'fenbi' and id > ? and id <= ?
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

                        sqlx::query("update paper set target_id=? where id=?")
                            .bind(paper.id)
                            .bind(source_id)
                            .execute(&self.source_db)
                            .await
                            .context("update source db label target_id failed")?;

                        progress.current = source_id;
                        *task = task.update_progress(&progress, &self.target_db).await?;
                    }
                    Err(e) => tracing::error!("find label failed: {:?}", e),
                };
            }
        }
        Ok(())
    }

    async fn save_paper(&self, paper: OriginPaper) -> anyhow::Result<paper::Model> {
        let source_paper_id = paper.id;
        let target_label_id: i32 = sqlx::query("select target_id from label where id = ?")
            .bind(paper.label_id)
            .fetch_one(&self.source_db)
            .await
            .with_context(|| format!("find target_id for label#{}", paper.label_id))?
            .try_get("target_id")
            .context("get target_id failed")?;
        let paper = paper.save_to(&self.target_db, target_label_id).await?;

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
            where from_ty = 'fenbi'
            and paper_id = ?
            order by number
            "##,
        )
        .bind(source_paper_id)
        .fetch_all(&self.source_db)
        .await
        .with_context(|| format!("find question_ids({source_paper_id}) failed"))?;

        // let qids = question_ids.iter().map(|q| q.question_id).collect_vec();

        // sqlx::query_as(r##"
        //     select

        // "##);

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
    ty: Option<i64>,
    chapters: Option<String>,
    id: i64,
    label_id: i64,
}

impl OriginPaper {
    async fn save_to<C: ConnectionTrait>(
        self,
        db: &C,
        label_id: i32,
    ) -> anyhow::Result<paper::Model> {
        let label = Label::find_by_id_with_cache(db, label_id)
            .await
            .with_context(|| format!("Label::find_by_id_with_cache({label_id}) failed"))?
            .expect(&format!("label#{label_id} not exists"));

        let year = pick_year(&self.date.expect("date is none")).expect("year not found");
        let mut active_model = paper::ActiveModel {
            title: Set(self.name.expect("name is none")),
            year: Set(year),
            exam_id: Set(label.exam_id),
            paper_type: Set(label.paper_type),
            label_id: Set(label.id),
            ..Default::default()
        };

        let chapters = &self.chapters.expect("chapters is none");
        let chapters: Vec<OriginChapter> =
            serde_json::from_str(chapters).context("parse chapters failed")?;

        let extra_value = if self.ty.expect("type is none") == 0 {
            let cs = Chapters {
                desc: None,
                chapters: chapters.into_iter().map(|m| m.into()).collect(),
            };
            serde_json::to_value(cs).context("Chapters to_value failed")?
        } else {
            let ec = EssayCluster {
                topic: self.topic,
                blocks: chapters.into_iter().map(|m| m.into()).collect(),
            };
            serde_json::to_value(ec).context("EssayCluster to_value failed")?
        };
        active_model.extra = Set(extra_value);

        let paper = active_model
            .insert_on_conflict(db)
            .await
            .context("insert paper failed")?;

        Ok(paper)
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginChapter {
    pub desc: String,
    pub name: String,
    pub question_count: i64,
}

impl Into<PaperChapter> for OriginChapter {
    fn into(self) -> PaperChapter {
        PaperChapter {
            desc: self.desc,
            name: self.name,
            count: self.question_count as i16,
        }
    }
}

impl Into<PaperBlock> for OriginChapter {
    fn into(self) -> PaperBlock {
        PaperBlock {
            desc: self.desc,
            name: self.name,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct QuestionIdNumber {
    question_id: i64,
    number: i32,
}

#[derive(Debug, sqlx::FromRow)]
struct OriginQuestions {
    ty: Option<i32>,
    content: Option<String>,
    accessories: Option<Vec<Accessory>>,
    material: Option<OriginMaterial>,
    keypoints: Option<Vec<OriginKeyPoint>>,
    correct_ratio: Option<String>,
    correct_answer: Option<String>,
    solution: Option<String>,
    solution_accessories: Option<Vec<Accessory>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OriginMaterial{
    pub id: i64,
    pub content: String,
    pub accessories: Option<Vec<Accessory>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Accessory {
    #[serde(rename = "type")]
    pub ty: i64,
    pub label: Option<String>,
    pub content: Option<String>,
    pub is_member_control: Option<i64>,
    pub url: Option<String>,
    pub audio_id: Option<String>,
    pub duration: Option<i64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OriginKeyPoint {
    pub id: i64,
    pub name: String,
}