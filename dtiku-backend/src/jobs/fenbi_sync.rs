use super::PaperSyncer;
use crate::jobs::JobScheduler;
use crate::plugins::embedding::Embedding;
use crate::utils::regex::pick_year;
use anyhow::Context;
use dtiku_base::model::schedule_task;
use dtiku_base::model::schedule_task::{Progress, TaskInstance};
use dtiku_paper::model::paper::EssayCluster;
use dtiku_paper::model::paper::PaperBlock;
use dtiku_paper::model::paper::PaperChapter;
use dtiku_paper::model::paper::{Chapters, PaperExtra};
use dtiku_paper::model::question;
use dtiku_paper::model::solution;
use dtiku_paper::model::solution::AnswerAnalysis;
use dtiku_paper::model::solution::FillBlank;
use dtiku_paper::model::solution::MultiChoice;
use dtiku_paper::model::solution::OtherAnswer;
use dtiku_paper::model::solution::SingleChoice;
use dtiku_paper::model::solution::StepAnalysis;
use dtiku_paper::model::solution::StepByStepAnswer;
use dtiku_paper::model::solution::TrueFalseChoice;
use dtiku_paper::model::FromType;
use dtiku_paper::model::Label;
use dtiku_paper::model::{exam_category, KeyPoint};
use dtiku_paper::model::{key_point, material};
use dtiku_paper::model::{label, ExamCategory};
use dtiku_paper::model::{paper, question_material};
use dtiku_paper::model::{paper_material, question_keypoint};
use futures::StreamExt;
use itertools::Itertools;
use scraper::Html;
use sea_orm::prelude::PgVector;
use sea_orm::{ActiveModelTrait, ColumnTrait, QueryFilter, Statement, TransactionTrait};
use sea_orm::{ConnectionTrait, EntityTrait};
use sea_orm::{PaginatorTrait, Set};
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use serde_with::{formats::CommaSeparator, serde_as, StringWithSeparator};
use spring::plugin::service::Service;
use spring::{async_trait, tracing};
use spring_sea_orm::DbConn;
use spring_sqlx::sqlx;
use spring_sqlx::ConnectPool;
use sqlx::types::Json;
use sqlx::Row;
use std::collections::HashMap;

static SINGLE_CHOICE: [i16; 1] = [1];
static MULTI_CHOICE: [i16; 1] = [2];
static INDEFINITE_CHOICE: [i16; 1] = [3];
static BLANK_CHOICE: [i16; 2] = [4, 6];
static TRUE_FALSE: [i16; 1] = [5];
static FILL_BLANK: [i16; 2] = [61, 64];
static STEP_BY_STEP_ANSWER: [i16; 13] = [11, 12, 16, 21, 22, 23, 24, 25, 26, 101, 102, 301, 302];
static CLOSED_ENDED_ANSWER: [i16; 1] = [13];
static OPEN_ENDED_ANSWER: [i16; 3] = [14, 15, 303];

#[derive(Clone, Service)]
#[service(prototype)]
pub struct FenbiSyncService {
    #[inject(component)]
    source_db: ConnectPool,
    #[inject(component)]
    target_db: DbConn,
    #[inject(component)]
    embedding: Embedding,
    task: schedule_task::Model,
    instance: TaskInstance,
}

impl PaperSyncer for FenbiSyncService {}

#[async_trait]
impl JobScheduler for FenbiSyncService {
    fn current_task(&mut self) -> &mut schedule_task::Model {
        &mut self.task
    }

    async fn inner_start(&mut self) -> anyhow::Result<()> {
        if self.save_exam_category().await? {
            return Ok(());
        }

        let mut progress = match &self.task.context {
            Value::Null => {
                let total = self
                    .total(
                        "select count(*) as total from label where from_ty='fenbi' and target_id is null",
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
                    "select count(*) as total from question_category where from_ty='fenbi'",
                    &self.source_db,
                )
                .await?;
            progress = Progress {
                name: "sync_categories".to_string(),
                total,
                current: 0,
            };
            self.task = self
                .task
                .update_progress(&progress, &self.target_db)
                .await?;
        }

        if progress.name == "sync_categories" {
            self.sync_categories(&mut progress).await?;

            let total = self
                .total(
                    "select max(id) as total from paper where from_ty='fenbi' and target_id is null",
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

impl FenbiSyncService {
    async fn sync_categories(&mut self, progress: &mut Progress<i64>) -> anyhow::Result<()> {
        if progress.total <= 0 {
            return Ok(());
        }
        let mut stream = sqlx::query_as::<_, OriginQuestionCategory>(
            r##"
                select
                    prefix,
                    extra
                from question_category
                where from_ty = 'fenbi'
                "##,
        )
        .fetch(&self.source_db);

        while let Some(row) = stream.next().await {
            match row {
                Ok(c) => {
                    let OriginQuestionCategory {
                        prefix: name,
                        extra,
                    } = c;
                    let paper_type = ExamCategory::find()
                        .filter(exam_category::Column::Prefix.eq(&name))
                        .one(&self.target_db)
                        .await?;

                    let (paper_type, exam_id) = match paper_type {
                        Some(p) => {
                            if p.pid != 0 {
                                let root_exam_type =
                                    ExamCategory::find_root_by_id(&self.target_db, p.pid)
                                        .await
                                        .with_context(|| {
                                            format!(
                                                "find root exam category failed:{} > {}",
                                                p.pid, p.id
                                            )
                                        })?
                                        .expect(&format!(
                                            "root_exam category not found:{} > {}",
                                            p.pid, p.id
                                        ));

                                (p.id, root_exam_type.id)
                            } else {
                                (p.id, p.id)
                            }
                        }
                        None => {
                            tracing::info!("find exam_category failed for fenbi#{name}");
                            continue;
                        }
                    };

                    self.target_db
                        .transaction::<_, (), anyhow::Error>(|tx| {
                            Box::pin(async move {
                                for c in extra.0 {
                                    Self::save_question_category_to_keypoint(
                                        c, 0, paper_type, exam_id, tx,
                                    )
                                    .await?;
                                }
                                Ok(())
                            })
                        })
                        .await?;

                    if progress.increase(1) {
                        self.task = self
                            .task
                            .update_progress(&progress, &self.target_db)
                            .await?;
                    }
                }
                Err(e) => tracing::error!("find categories failed: {:?}", e),
            }
        }
        Ok(())
    }

    async fn save_question_category_to_keypoint<C: ConnectionTrait>(
        c: QuestionCategory,
        parent_id: i32,
        paper_type: i16,
        exam_id: i16,
        target_db: &C,
    ) -> anyhow::Result<()> {
        let kp = key_point::ActiveModel {
            name: Set(c.name),
            pid: Set(parent_id),
            paper_type: Set(paper_type),
            exam_id: Set(exam_id),
            ..Default::default()
        }
        .insert_on_conflict(target_db)
        .await?;

        let c_parent_id = kp.id;

        if let Some(cs) = c.children {
            for c in cs {
                Box::pin(async move {
                    Self::save_question_category_to_keypoint(
                        c,
                        c_parent_id,
                        paper_type,
                        exam_id,
                        target_db,
                    )
                    .await
                })
                .await?;
            }
        }

        Ok(())
    }

    async fn sync_label(&mut self, progress: &mut Progress<i64>) -> anyhow::Result<()> {
        if progress.total <= 0 {
            return Ok(());
        }

        let mut stream = sqlx::query_as::<_, OriginLabel>(r##"
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
        where from_ty = 'fenbi' and target_id is null
        order by exam_root,exam_name,paper_type,jsonb_extract_path_text(extra,'parent','name') is not null,id,label_name
        "##).fetch(&self.source_db);

        while let Some(row) = stream.next().await {
            match row {
                Ok(row) => {
                    let source_id = row.id;
                    let label = row.save_to(&self.target_db).await?;

                    sqlx::query("update label set target_id=$1 where id=$2 and from_ty='fenbi'")
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

    async fn save_exam_category(&mut self) -> anyhow::Result<bool> {
        if ExamCategory::find()
            .filter(exam_category::Column::FromTy.eq(FromType::Fenbi))
            .count(&self.target_db)
            .await?
            > 0
        {
            tracing::info!("exam_category already exists");
            return Ok(false);
        }
        let category = sqlx::query_as::<_, OriginExamCategory>(
            r##"
                select extra
                from exam_category_tree
                where from_ty = 'fenbi'
                "##,
        )
        .fetch_one(&self.source_db)
        .await
        .context("fetch fenbi exam_category_tree failed")?;

        for c in category.extra.0 {
            let root_category = c
                .live_config_item
                .to_exam_category()
                .insert_on_conflict(&self.target_db)
                .await
                .context("insert exam category failed")?;

            let second_category = c
                .course_set
                .to_exam_category(root_category.id)
                .insert_on_conflict(&self.target_db)
                .await
                .context("insert exam category failed")?;

            for c in c.courses {
                if c.prefix != second_category.prefix {
                    c.to_exam_category(second_category.id)
                        .insert_on_conflict(&self.target_db)
                        .await
                        .context("insert exam category failed")?;
                }
            }
        }

        Ok(true)
    }

    async fn sync_paper(&mut self, progress: &mut Progress<i64>) -> anyhow::Result<()> {
        while progress.current < progress.total {
            let current = progress.current;
            let next_step_id: i64 = current + 1000;
            let mut stream = sqlx::query_as::<_, OriginPaper>(
                r##"
                    select
                        jsonb_extract_path_text(extra,'name') as name,
                        jsonb_extract_path_text(extra,'date') as date,
                        jsonb_extract_path_text(extra,'topic') as topic,
                        (jsonb_extract_path(extra,'type'))::int as ty,
                        jsonb_extract_path_text(extra,'chapters') as chapters,
                        id,
                        label_id
                    from paper
                    where from_ty = 'fenbi' and target_id is null and id > $1 and id <= $2
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

                        sqlx::query(
                            "update paper set target_id=$1 where id=$2 and from_ty='fenbi'",
                        )
                        .bind(paper.id)
                        .bind(source_id)
                        .execute(&self.source_db)
                        .await
                        .context("update source db paper target_id failed")?;

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
        let target_label_id: i32 =
            sqlx::query("select target_id from label where id = $1 and from_ty='fenbi'")
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
            and paper_id = $1
            order by number
            "##,
        )
        .bind(source_paper_id)
        .fetch_all(&self.source_db)
        .await
        .with_context(|| format!("find question_ids({source_paper_id}) failed"))?;

        let qids = question_ids.iter().map(|q| q.question_id).collect_vec();
        let qid_num_map: HashMap<_, _> = question_ids
            .into_iter()
            .map(|q| (q.question_id, q.number + 1))
            .collect();

        let questions = sqlx::query_as::<_, OriginQuestion>(
            r##"
            select
                id,
                target_id,
                (jsonb_extract_path(extra,'type'))::int2 as ty,
                jsonb_extract_path_text(extra,'content') as content,
                jsonb_extract_path(extra,'accessories') as accessories,
                (jsonb_extract_path(extra,'questionMeta','correctRatio'))::real as correct_ratio,
                nullif(jsonb_extract_path(extra,'correctAnswer'), 'null') as correct_answer,
                jsonb_extract_path_text(extra,'solution') as solution,
                jsonb_extract_path(extra,'solutionAccessories') as solution_accessories,
                nullif(jsonb_extract_path(extra,'material'), 'null') as material,
                nullif(jsonb_extract_path(extra,'keypoints'), 'null') as keypoints
            from question
            where from_ty = 'fenbi'
            and id = any($1)
        "##,
        )
        .bind(qids)
        .fetch_all(&self.source_db)
        .await
        .with_context(|| format!("find questions({source_paper_id}) failed"))?;

        let material_ids: Vec<MaterialIdNumber> = sqlx::query_as(
            r##"
            select
                material_id,
                number
            from paper_material
            where from_ty = 'fenbi'
            and paper_id = $1
            order by number
            "##,
        )
        .bind(source_paper_id)
        .fetch_all(&self.source_db)
        .await
        .with_context(|| format!("find material_ids({source_paper_id}) failed"))?;

        let mids = material_ids.iter().map(|m| m.material_id).collect_vec();
        let mid_num_map: HashMap<_, _> = material_ids
            .into_iter()
            .map(|m| (m.material_id, m.number + 1))
            .collect();

        let materials = sqlx::query_as::<_, OriginMaterial>(
            r##"
            select
                id,
                target_id,
                jsonb_extract_path_text(extra,'content') as content,
                nullif(jsonb_extract_path(extra,'accessories'), 'null') as accessories
            from material
            where from_ty = 'fenbi'
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
        for m in materials {
            let num = mid_num_map
                .get(&m.id)
                .expect("mid is not exists in mid_num_map");
            self.save_material(m, paper.id, *num).await?;
        }

        for q in questions {
            let correct_ratio = q.correct_ratio;
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
            solution.question_id = Set(q_in_db.id);
            solution
                .insert(&self.target_db)
                .await
                .context("insert solution failed")?;

            if let Some(m) = q.material {
                let origin_m_id = m.id;
                let target_id: Option<i32> = sqlx::query_scalar::<_, Option<i32>>(
                    "select target_id from material where from_ty = 'fenbi' and id = $1",
                )
                .bind(origin_m_id)
                .fetch_optional(&self.source_db)
                .await
                .with_context(|| format!("select material target_id by id#{origin_m_id}"))?
                .flatten();

                if let Some(target_m_id) = target_id {
                    question_material::ActiveModel {
                        question_id: Set(q_in_db.id),
                        material_id: Set(target_m_id),
                    }
                    .insert_on_conflict(&self.target_db)
                    .await
                    .context("insert question_material failed")?;
                } else {
                    tracing::warn!(
                        "origin_question#{} ==> question#{}: material#{} target_id not exists",
                        q.id,
                        q_in_db.id,
                        origin_m_id
                    );
                    let num = mid_num_map
                        .get(&m.id)
                        .expect("mid is not exists in mid_num_map");
                    self.save_material(m.0, paper.id, *num).await?;
                }
            }

            let keypoint_path = match q.keypoints {
                Some(keypoints) => {
                    let mut keypoint_ids = vec![];
                    for kp in keypoints.0 {
                        let paper_type = paper.paper_type;
                        let keypoint_name = kp.name;
                        let kp = KeyPoint::find_by_paper_type_and_name(
                            &self.target_db,
                            paper_type,
                            &keypoint_name,
                        )
                        .await
                        .with_context(|| {
                            format!("find paper_type#{paper_type} keypoint({keypoint_name}) failed")
                        })?;

                        if let Some(keypoint) = kp {
                            question_keypoint::ActiveModel {
                                question_id: Set(q_in_db.id),
                                key_point_id: Set(keypoint.id),
                                year: Set(paper.year),
                            }
                            .insert_on_conflict(&self.target_db)
                            .await
                            .context("insert question_keypoint failed")?;
                            keypoint_ids.push(keypoint.id);
                        }
                    }
                    KeyPoint::query_common_keypoint_path(&self.target_db, &keypoint_ids).await?
                }
                None => {
                    let paper_type = paper.paper_type;
                    if let Some(chapter_name) = paper.extra.compute_chapter_name(*num) {
                        let kp = KeyPoint::find_by_paper_type_and_name(
                            &self.target_db,
                            paper_type,
                            &chapter_name,
                        )
                        .await
                        .with_context(|| {
                            format!("find paper_type#{paper_type} keypoint({chapter_name}) failed")
                        })?;
                        if let Some(keypoint) = kp {
                            KeyPoint::query_common_keypoint_path(
                                &self.target_db,
                                &vec![keypoint.id],
                            )
                            .await?
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
            };

            // ltree
            let stmt = match &keypoint_path {
                Some(path) => Statement::from_sql_and_values(
                    sea_orm::DatabaseBackend::Postgres,
                    r#"INSERT INTO paper_question (paper_id, question_id, sort, paper_type, keypoint_path, correct_ratio)
                        VALUES ($1, $2, $3, $4, CAST($5 AS ltree), $6)
                        ON CONFLICT (paper_id, question_id)
                        DO UPDATE SET 
                            sort=EXCLUDED.sort, 
                            paper_type=EXCLUDED.paper_type, 
                            keypoint_path=EXCLUDED.keypoint_path, 
                            correct_ratio=EXCLUDED.correct_ratio
                        "#,
                    vec![
                        paper.id.into(),
                        q_in_db.id.into(),
                        (*num as i16).into(),
                        q_in_db.paper_type.into(),
                        path.into(),
                        correct_ratio.into(),
                    ],
                ),
                None => Statement::from_sql_and_values(
                    sea_orm::DatabaseBackend::Postgres,
                    r#"INSERT INTO paper_question (paper_id, question_id, sort, paper_type, correct_ratio)
                        VALUES ($1, $2, $3, $4, $5)
                        ON CONFLICT (paper_id, question_id)
                        DO UPDATE SET 
                            sort=EXCLUDED.sort, 
                            paper_type=EXCLUDED.paper_type, 
                            correct_ratio=EXCLUDED.correct_ratio
                        "#,
                    vec![
                        paper.id.into(),
                        q_in_db.id.into(),
                        (*num as i16).into(),
                        q_in_db.paper_type.into(),
                        correct_ratio.into(),
                    ],
                ),
            };

            self.target_db.execute(stmt).await.with_context(|| {
                format!(
                    "insert paper_question failed, key_point_path:{:?}",
                    keypoint_path
                )
            })?;
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
        let m_in_db = material
            .insert_on_conflict(&self.target_db)
            .await
            .context("insert paper_material failed")?;
        paper_material::ActiveModel {
            paper_id: Set(paper_id),
            material_id: Set(m_in_db.id),
            sort: Set(num as i16),
        }
        .insert_on_conflict(&self.target_db)
        .await
        .context("insert paper_material failed")?;
        sqlx::query("update material set target_id=$1 where id=$2 and from_ty='fenbi'")
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
        let id = self.id;
        let name = self.label_name.clone();

        let root = exam_category::ActiveModel {
            pid: Set(0),
            name: Set(self.exam_root.expect("exam_root is none")),
            prefix: Set(self.exam_root_prefix.expect("exam_root_prefix is none")),
            from_ty: Set(FromType::Fenbi),
            ..Default::default()
        }
        .insert_on_conflict(db)
        .await
        .context(format!(
            "insert root exam_category failed:#{id:?}--->{name:?}"
        ))?;

        let second = exam_category::ActiveModel {
            pid: Set(root.id),
            name: Set(self.exam_name.expect("exam_name is none")),
            prefix: Set(self.exam_prefix.clone().expect("exam_prefix is none")),
            from_ty: Set(FromType::Fenbi),
            ..Default::default()
        }
        .insert_on_conflict(db)
        .await
        .context(format!(
            "insert second exam_category failed:#{id:?}--->{name:?}"
        ))?;

        let leaf = if self.exam_prefix != self.paper_prefix {
            exam_category::ActiveModel {
                pid: Set(second.id),
                name: Set(self.paper_type.expect("paper_type is none")),
                prefix: Set(self.paper_prefix.expect("paper_prefix is none")),
                from_ty: Set(FromType::Fenbi),
                ..Default::default()
            }
            .insert_on_conflict(db)
            .await
            .context(format!(
                "insert leaf exam_category failed:#{id:?}--->{name:?}"
            ))?
        } else {
            second
        };

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
                    .context(format!("insert parent label failed:#{id:?}--->{name:?}"))?,
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
            .context(format!("insert label failed:#{id:?}--->{name:?}"))?;
        Ok(label)
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
    async fn save_to<C: ConnectionTrait>(
        self,
        db: &C,
        label_id: i32,
    ) -> anyhow::Result<paper::Model> {
        let label = Label::find_by_id(label_id)
            .one(db)
            .await
            .with_context(|| format!("Label::find_by_id({label_id}) failed"))?
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
            PaperExtra::Chapters(cs)
        } else {
            let ec = EssayCluster {
                topic: self.topic,
                blocks: chapters.into_iter().map(|m| m.into()).collect(),
            };
            PaperExtra::EssayCluster(ec)
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

#[derive(Debug, Clone, sqlx::FromRow)]
struct OriginQuestion {
    id: i64,
    target_id: Option<i32>,
    ty: i16,
    content: String,
    accessories: Json<Vec<QuestionAccessory>>,
    material: Option<Json<OriginMaterial>>,
    keypoints: Option<Json<Vec<OriginKeyPoint>>>,
    correct_ratio: Option<f32>,
    correct_answer: Option<Json<CorrectAnswer>>,
    solution: Option<String>,
    solution_accessories: Json<Vec<SolutionAccessory>>,
}

impl OriginQuestion {
    async fn to_question(&self, model: &Embedding) -> anyhow::Result<question::ActiveModel> {
        let Self {
            id, ty, content, ..
        } = self;
        let mut options_string = String::new();
        let extra = if SINGLE_CHOICE.contains(ty) {
            let list = self.filter_accessory(|a| [101i16, 102i16].contains(&a.ty));
            let os = list.last().expect(&format!(
                "q#{id} SingleChoice don't contains 101/102 options:{list:?}"
            ));
            let options = os
                .options
                .clone()
                .expect("SingleChoice 101/102 options is none");
            options_string = options.iter().join("\n");
            question::QuestionExtra::SingleChoice { options }
        } else if MULTI_CHOICE.contains(ty) {
            let list = self.filter_accessory(|a| [101i16, 102i16].contains(&a.ty));
            let os = list.last().expect(&format!(
                "q#{id} MultiChoice don't contains 101/102 options:{list:?}"
            ));
            let options = os.options.clone().expect(&format!(
                "q#{id} MultiChoice 101/102 options is none:{list:?}"
            ));
            options_string = options.iter().join("\n");
            question::QuestionExtra::MultiChoice { options }
        } else if INDEFINITE_CHOICE.contains(ty) {
            let list = self.filter_accessory(|a| [101i16, 102i16].contains(&a.ty));
            let os = list.last().expect(&format!(
                "q#{id} IndefiniteChoice don't contains 101/102 options:{list:?}"
            ));
            let options = os.options.clone().expect(&format!(
                "q#{id} IndefiniteChoice 101/102 options is none:{list:?}"
            ));
            options_string = options.iter().join("\n");
            question::QuestionExtra::IndefiniteChoice { options }
        } else if BLANK_CHOICE.contains(ty) {
            let list = self.filter_accessory(|a| [101i16, 102i16].contains(&a.ty));
            let os = list.last().expect(&format!(
                "q#{id} BlankChoice don't contains 101/102 options:{list:?}"
            ));
            let options = os.options.clone().expect(&format!(
                "q#{id} BlankChoice 101/102 options is none:{list:?}"
            ));
            options_string = options.iter().join("\n");
            question::QuestionExtra::BlankChoice { options }
        } else if TRUE_FALSE.contains(ty) {
            question::QuestionExtra::TrueFalse
        } else if STEP_BY_STEP_ANSWER.contains(ty) {
            let list = self.filter_accessory(|a| [182i16].contains(&a.ty));
            let os = list.last().expect(&format!(
                "q#{id} BlankChoice don't contains 182 options:{list:?}"
            ));
            question::QuestionExtra::StepByStepQA(question::QA {
                title: content.clone(),
                word_count: os.word_count,
                material_ids: os.material_indexes.clone(),
            })
        } else if CLOSED_ENDED_ANSWER.contains(ty) {
            let list = self.filter_accessory(|a| [182i16].contains(&a.ty));
            let os = list.last().expect(&format!(
                "q#{id} ClosedEndedQA don't contains 182 options:{list:?}"
            ));
            question::QuestionExtra::ClosedEndedQA(question::QA {
                title: os
                    .title
                    .clone()
                    .expect(&format!("q#{id} StepByStepQA title is none:{list:?}")),
                word_count: os.word_count,
                material_ids: os.material_indexes.clone(),
            })
        } else if OPEN_ENDED_ANSWER.contains(ty) {
            let list = self.filter_accessory(|a| [182i16].contains(&a.ty));
            let os = list.last().expect(&format!(
                "q#{id} BlankChoice don't contains 182 options:{list:?}"
            ));
            question::QuestionExtra::ClosedEndedQA(question::QA {
                title: os
                    .title
                    .clone()
                    .expect(&format!("q#{id} StepByStepQA title is none:{list:?}")),
                word_count: os.word_count,
                material_ids: os.material_indexes.clone(),
            })
        } else if FILL_BLANK.contains(ty) {
            let list = self.filter_accessory(|a| [182i16].contains(&a.ty));
            let os = list.last().expect(&format!(
                "q#{id} BlankChoice don't contains 182 options:{list:?}"
            ));
            question::QuestionExtra::ClosedEndedQA(question::QA {
                title: os
                    .title
                    .clone()
                    .expect(&format!("q#{id} StepByStepQA title is none:{list:?}")),
                word_count: os.word_count,
                material_ids: os.material_indexes.clone(),
            })
        } else {
            Err(anyhow::anyhow!("ty#{ty} is not defined"))?
        };
        let txt = {
            // scraper::Html 底层用了 tendril::NonAtomic 和 Cell 类型，而这些类型不是线程安全的，所以它 不实现 Send。
            // 用代码块，让html 变量在这里作用域结束，释放掉 Cell 的引用。
            let html = Html::parse_fragment(&format!("{content}\n{options_string}"));
            html.root_element().text().collect::<String>()
        };
        let embedding = model.text_embedding(&txt).await?;

        let mut m = question::ActiveModel {
            content: Set(content.into()),
            extra: Set(extra),
            embedding: Set(PgVector::from(embedding)),
            ..Default::default()
        };
        if let Some(target_id) = self.target_id {
            m.id = Set(target_id);
        }
        Ok(m)
    }

    fn to_solution(&self) -> anyhow::Result<solution::ActiveModel> {
        let Self {
            id,
            ty,
            correct_answer,
            solution,
            solution_accessories,
            ..
        } = self;
        let correct_answer = correct_answer.clone();
        let extra = if SINGLE_CHOICE.contains(ty) {
            solution::SolutionExtra::SingleChoice(SingleChoice {
                answer: correct_answer
                    .expect(&format!("q#{id} correct_answer is none"))
                    .choice
                    .clone()
                    .expect(&format!("q#{id} correct_answer.choice is none"))
                    .remove(0),
                analysis: solution.clone().expect(&format!("q#{id} solution is none")),
            })
        } else if MULTI_CHOICE.contains(ty) {
            let answer = correct_answer
                .expect(&format!("q#{id} correct_answer is none"))
                .choice
                .clone()
                .expect(&format!("q#{id} correct_answer.choice is none"));
            solution::SolutionExtra::MultiChoice(MultiChoice {
                answer,
                analysis: solution.clone().expect(&format!("q#{id} solution is none")),
            })
        } else if INDEFINITE_CHOICE.contains(ty) {
            let answer = correct_answer
                .expect(&format!("q#{id} correct_answer is none"))
                .choice
                .clone()
                .expect(&format!("q#{id} correct_answer.choice is none"));
            solution::SolutionExtra::IndefiniteChoice(MultiChoice {
                answer,
                analysis: solution.clone().expect(&format!("q#{id} solution is none")),
            })
        } else if BLANK_CHOICE.contains(ty) {
            solution::SolutionExtra::BlankChoice(SingleChoice {
                answer: correct_answer
                    .expect(&format!("q#{id} correct_answer is none"))
                    .choice
                    .clone()
                    .expect(&format!("q#{id} correct_answer.choice is none"))
                    .remove(0),
                analysis: solution.clone().expect(&format!("q#{id} solution is none")),
            })
        } else if TRUE_FALSE.contains(ty) {
            solution::SolutionExtra::TrueFalse(TrueFalseChoice {
                answer: correct_answer
                    .expect(&format!("q#{id} correct_answer is none"))
                    .choice
                    .clone()
                    .expect(&format!("q#{id} correct_answer.choice is none"))
                    .remove(0)
                    == 0,
                analysis: solution.clone().expect(&format!("q#{id} solution is none")),
            })
        } else if FILL_BLANK.contains(ty) {
            let blanks = correct_answer
                .expect(&format!("q#{id} correct_answer is none"))
                .blanks
                .clone()
                .expect(&format!("q#{id} correct_answer.blanks is none"));
            solution::SolutionExtra::FillBlank(FillBlank {
                blanks,
                analysis: solution.clone().expect(&format!("q#{id} solution is none")),
            })
        } else {
            if solution_accessories.len() < 1 && correct_answer.is_some() {
                solution::SolutionExtra::ClosedEndedQA(AnswerAnalysis {
                    analysis: solution.clone().expect(&format!("q#{id} solution is none")),
                    answer: correct_answer
                        .expect(&format!("q#{id} correct_answer is none"))
                        .answer
                        .clone()
                        .expect(&format!("q#{id} correct_answer.answer is none")),
                })
            } else if solution_accessories.len() > 1
                && correct_answer.clone().is_none_or(|c| c.answer.is_none())
            {
                let analysis = solution_accessories
                    .0
                    .iter()
                    .map(|a| a.convert_into())
                    .collect();
                solution::SolutionExtra::OpenEndedQA(StepByStepAnswer {
                    solution: solution.clone(),
                    analysis,
                })
            } else {
                let analysis = solution_accessories
                    .0
                    .iter()
                    .map(|a| a.convert_into())
                    .collect();
                solution::SolutionExtra::OtherQA(OtherAnswer {
                    answer: correct_answer.and_then(|ca| ca.answer.clone()),
                    solution: solution.clone(),
                    analysis,
                })
            }
        };
        Ok(solution::ActiveModel {
            extra: Set(extra),
            ..Default::default()
        })
    }

    fn filter_accessory<F>(&self, filter: F) -> Vec<&QuestionAccessory>
    where
        F: Fn(&QuestionAccessory) -> bool,
    {
        self.accessories
            .0
            .iter()
            .filter(|a| filter(a))
            .collect_vec()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct QuestionAccessory {
    #[serde(rename = "type")]
    pub ty: i16,
    pub options: Option<Vec<String>>,
    pub name: Option<String>,
    pub label: Option<String>,
    pub content: Option<String>,
    pub is_member_control: Option<i64>,
    pub score: Option<f64>,
    pub title: Option<String>,
    pub blank_type: Option<i64>,
    pub word_count: Option<i16>,
    #[serde(default)]
    pub material_indexes: Vec<i32>,
    pub url: Option<String>,
    pub audio_id: Option<String>,
    pub duration: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SolutionAccessory {
    #[serde(rename = "type")]
    pub ty: i64,
    pub label: String,
    pub content: String,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CorrectAnswer {
    #[serde(rename = "type")]
    pub ty: i16,
    #[serde_as(as = "Option<StringWithSeparator::<CommaSeparator, u8>>")]
    pub choice: Option<Vec<u8>>,
    pub blanks: Option<Vec<String>>,
    pub answer: Option<String>,
}

#[derive(Debug, Clone, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct OriginMaterial {
    pub id: i64,
    pub target_id: Option<i32>,
    pub content: String,
    pub accessories: Option<Json<Vec<MaterialAccessory>>>,
}

impl TryInto<material::ActiveModel> for OriginMaterial {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<material::ActiveModel, Self::Error> {
        let extra = match self.accessories {
            Some(a) => Some(
                a.0.into_iter()
                    .map(|a| a.try_into())
                    .collect::<anyhow::Result<Vec<material::MaterialExtra>>>()?,
            ),
            None => None,
        };
        let mut am = material::ActiveModel {
            content: Set(self.content),
            extra: Set(extra.unwrap_or_default()),
            ..Default::default()
        };
        if let Some(id) = self.target_id {
            am.id = Set(id);
        }
        Ok(am)
    }
}

impl TryInto<material::MaterialExtra> for MaterialAccessory {
    type Error = anyhow::Error;
    fn try_into(self) -> Result<material::MaterialExtra, Self::Error> {
        match self.ty {
            151 => Ok(material::MaterialExtra::Translation(
                self.translation.expect("translation is none"),
            )),
            181 => match self.label.expect("label is none").as_str() {
                "materialExplain" => Ok(material::MaterialExtra::MaterialExplain(
                    self.content.expect("materialExplain content is none"),
                )),
                "transcript" => Ok(material::MaterialExtra::Transcript(
                    self.content.expect("transcript content is none"),
                )),
                "zdch" => Ok(material::MaterialExtra::Dictionary(
                    self.content.expect("zdch content is none"),
                )),
                _unknown => Err(anyhow::anyhow!("unknown material label:{_unknown}")),
            },
            185 => Ok(material::MaterialExtra::Audio(
                self.url.expect("Audio url is none"),
            )),
            _unknown => Err(anyhow::anyhow!(
                "unknown material accessory type:{_unknown}"
            )),
        }
    }
}

impl SolutionAccessory {
    fn convert_into(&self) -> StepAnalysis {
        StepAnalysis {
            label: self.label.clone(),
            content: self.content.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialAccessory {
    #[serde(rename = "type")]
    pub ty: i64,
    pub label: Option<String>,
    pub content: Option<String>,
    pub translation: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OriginKeyPoint {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, sqlx::FromRow)]
struct MaterialIdNumber {
    material_id: i64,
    number: i32,
}

#[derive(Debug, sqlx::FromRow)]
struct OriginQuestionCategory {
    pub prefix: String,
    pub extra: Json<Vec<QuestionCategory>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct QuestionCategory {
    pub name: String,
    pub count: i64,
    #[serde(rename = "type")]
    pub type_field: Option<i64>,
    pub ptype: Option<i64>,
    pub children: Option<Vec<QuestionCategory>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
pub struct OriginExamCategory {
    pub extra: Json<Vec<CourseCategory>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseCategory {
    pub icon: String,
    pub courses: Vec<Course>,
    pub course_set: CourseSet,
    pub live_config_item: LiveConfigItem,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Course {
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub type_field: i64,
    pub hidden: bool,
    pub prefix: String,
    pub modules: Vec<Value>,
    pub printable: bool,
    pub scannable: bool,
    pub top_column: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseSet {
    pub id: i64,
    pub name: String,
    pub prefix: String,
    pub ordinal: i64,
    pub course_ids: Vec<i64>,
    pub multi_quiz: bool,
    pub created_time: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiveConfigItem {
    pub id: i64,
    pub name: String,
    pub prefix: String,
}

impl LiveConfigItem {
    fn to_exam_category(self) -> exam_category::ActiveModel {
        exam_category::ActiveModel {
            name: Set(self.name),
            prefix: Set(self.prefix),
            pid: Set(0),
            from_ty: Set(FromType::Fenbi),
            ..Default::default()
        }
    }
}

impl CourseSet {
    fn to_exam_category(self, pid: i16) -> exam_category::ActiveModel {
        exam_category::ActiveModel {
            name: Set(self.name),
            prefix: Set(self.prefix),
            pid: Set(pid),
            from_ty: Set(FromType::Fenbi),
            ..Default::default()
        }
    }
}

impl Course {
    fn to_exam_category(self, pid: i16) -> exam_category::ActiveModel {
        exam_category::ActiveModel {
            name: Set(self.name),
            prefix: Set(self.prefix),
            pid: Set(pid),
            from_ty: Set(FromType::Fenbi),
            ..Default::default()
        }
    }
}
