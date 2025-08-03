use super::{JobScheduler, PaperSyncer};
use crate::jobs::{MaterialIdNumber, QuestionIdNumber};
use crate::plugins::embedding::Embedding;
use anyhow::Context;
use dtiku_base::model::schedule_task::{self, Progress, TaskInstance};
use dtiku_paper::model::paper::{Chapters, EssayCluster, PaperChapter, PaperExtra};
use dtiku_paper::model::{
    exam_category, label, material, paper, paper_material, question, question_keypoint, solution,
    ExamCategory, FromType, KeyPoint, Label,
};
use futures::StreamExt;
use itertools::Itertools;
use pinyin::ToPinyin;
use sea_orm::ActiveModelTrait;
use sea_orm::{ActiveValue::Set, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter};
use sea_orm::{ColumnTrait, Statement};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use spring::{async_trait, plugin::service::Service, tracing};
use spring_sea_orm::DbConn;
use spring_sqlx::ConnectPool;
use sqlx::types::Json;
use sqlx::Row;
use std::collections::HashMap;

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

        let mut stream = sqlx::query_as::<_, OriginLabel>(
            r##"
                select
                    id,
                    jsonb_extract_path_text(extra,'name') as name,
                    jsonb_extract_path_text(extra,'parent_name') as parent_name
                from "label" l
                where from_ty ='huatu'
        "##,
        )
        .fetch(&self.source_db);

        while let Some(row) = stream.next().await {
            match row {
                Ok(row) => {
                    let source_id = row.id;
                    let exam_id = row.select_from(&self.target_db).await?;

                    sqlx::query("update label set target_id=$1 where id=$2 and from_ty='huatu'")
                        .bind(exam_id)
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
                            coalesce(jsonb_extract_path_text(extra,'year'), jsonb_extract_path_text(extra,'paperYear'))::integer as year,
                            jsonb_extract_path_text(extra,'modules') as modules,
                            jsonb_extract_path_text(extra,'topicNameList') as topics
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
        let source_paper_id = paper.id;
        let target_exam_id: i32 =
            sqlx::query("select target_id from label where id = $1 and from_ty='huatu'")
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
            where from_ty = 'huatu'
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
            where from_ty = 'fenbi'
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
        let mids = mid_num_map.keys().cloned().collect_vec();

        let questions = sqlx::query_as::<_, OriginQuestion>(
            r##"
            select
                id,
                target_id,
                (jsonb_extract_path(extra,'area'))::int2 as area,
                (jsonb_extract_path(extra,'year'))::int2 as year,
                jsonb_extract_path_text(extra,'teachType') as ty,
                jsonb_extract_path_text(extra,'stem') as content,
                jsonb_extract_path(extra,'choices') as choices,
                (jsonb_extract_path(extra,'difficult'))::real as difficult,
                nullif(jsonb_extract_path(extra,'answerList'), 'null') as answer_list,
                jsonb_extract_path_text(extra,'analysis') as analysis,
                jsonb_extract_path_text(extra,'extend') as extend,
                jsonb_extract_path_text(extra,'answerRequire') as answer_require,
                jsonb_extract_path_text(extra,'referAnalysis') as refer_analysis,
                nullif(jsonb_extract_path_text(extra,'material'), 'null') as material,
                nullif(jsonb_extract_path(extra,'pointsName'), 'null') as points_name
            from question
            where from_ty = 'huatu'
            and id = any($1)
        "##,
        )
        .bind(qids)
        .fetch_all(&self.source_db)
        .await
        .with_context(|| format!("find questions({source_paper_id}) failed"))?;

        let materials = sqlx::query_as::<_, OriginMaterial>(
            r##"
            select
                id,
                target_id,
                jsonb_extract_path_text(extra,'content') as content
            from material
            where from_ty = 'huatu'
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

        for mut q in questions {
            // let correct_ratio = q.correct_ratio;
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

            let origin_m_id = 1;
            if let Some(m) = q.material {
                let target_id: Option<i32> = sqlx::query_scalar::<_, Option<i32>>(
                    "select target_id from material where from_ty = 'fenbi' and id = $1",
                )
                .bind(origin_m_id)
                .fetch_optional(&self.source_db)
                .await
                .with_context(|| format!("select material target_id by id#{origin_m_id}"))?
                .flatten();
            }

            let keypoint_path = match q.points_name {
                Some(keypoints) => {
                    let mut keypoint_ids = vec![];
                    for keypoint_name in keypoints.0 {
                        let paper_type = paper.paper_type;
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

            let correct_ratio = 1.0;
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
        let m_in_db = material.insert_on_conflict(&self.target_db).await?;
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
    async fn select_from<C: ConnectionTrait>(self, db: &C) -> anyhow::Result<Option<i16>> {
        let stmt = Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            r##"
            select ec2.id as id from exam_category ec1
            left join exam_category ec2 
            on ec1.id = ec2.pid
            where ec2."name" = $1
            and ec1."name" = $2
            "##,
            vec![self.name.clone().into(), self.parent_name.clone().into()],
        );

        let r = db.query_one(stmt).await.with_context(|| {
            format!(
                "query exam_category failed, name:{:?}<<parent_name:{:?}",
                self.name, self.parent_name
            )
        })?;
        Ok(match r {
            Some(qr) => {
                let id: i16 = qr.try_get("", "id").context("get id column failed")?;
                Some(id)
            }
            None => None,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct OriginPaper {
    area: Option<String>,
    name: Option<String>,
    ty: i32,
    year: Option<i32>,
    modules: Option<Json<Vec<PaperBlock>>>,
    topics: Option<Json<Vec<String>>>,
    id: i64,
    label_id: i64,
}

impl OriginPaper {
    async fn save_paper<C: ConnectionTrait>(
        self,
        db: &C,
        exam_paper_id: i32,
    ) -> anyhow::Result<paper::Model> {
        let extra = if self.ty > 0 {
            let chapters = self
                .modules
                .expect(&format!("paper#{} modules 不存在", self.id));
            let cs = Chapters {
                desc: None,
                chapters: chapters.iter().map(|m: &PaperBlock| m.into()).collect(),
            };
            PaperExtra::Chapters(cs)
        } else {
            let topics = self.topics.map(|ts| ts.iter().join(","));
            let ec = EssayCluster {
                topic: topics,
                blocks: vec![],
            };
            PaperExtra::EssayCluster(ec)
        };
        let paper_type = exam_paper_id as i16;
        let exam = ExamCategory::find_root_by_id(db, paper_type)
            .await?
            .expect(&format!(
                "paper_type#{} exam root_id not found",
                exam_paper_id
            ));
        let area = self.area.expect(&format!("paper#{} area 不存在", self.id));
        let label =
            Label::find_by_exam_id_and_paper_type_and_name(db, exam.id, paper_type, &area).await?;
        let label = match label {
            Some(l) => l,
            None => label::ActiveModel {
                name: Set(area),
                pid: Set(0),
                exam_id: Set(exam.id),
                paper_type: Set(paper_type),
                hidden: Set(false),
                ..Default::default()
            }
            .insert_on_conflict(db)
            .await
            .context("label insert failed")?,
        };
        paper::ActiveModel {
            year: Set(self.year.expect(&format!("paper#{} year 不存在", self.id)) as i16),
            title: Set(self.name.expect(&format!("paper#{} name 不存在", self.id))),
            exam_id: Set(exam.id),
            paper_type: Set(paper_type),
            label_id: Set(label.id),
            extra: Set(extra),
            ..Default::default()
        }
        .insert_on_conflict(db)
        .await
        .context("paper insert failed")
    }
}

#[derive(Debug, Deserialize)]
struct PaperBlock {
    name: String,
    qcount: i32,
}

impl Into<PaperChapter> for &PaperBlock {
    fn into(self) -> PaperChapter {
        PaperChapter {
            name: self.name.clone(),
            desc: "".to_string(),
            count: self.qcount as i16,
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct OriginQuestion {
    id: i64,
    target_id: Option<i32>,
    area: i16,
    year: i16,
    ty: Option<String>,
    content: String,
    choices: Json<Vec<String>>,
    difficult: f32,
    answer_list: Json<Vec<String>>,
    analysis: Option<String>,
    extend: Option<String>,
    answer_require: Option<String>,
    refer_analysis: Option<String>,
    material: Option<String>,
    points_name: Option<Json<Vec<String>>>,
}

impl OriginQuestion {
    async fn to_question(&self, model: &Embedding) -> anyhow::Result<question::ActiveModel> {
        todo!()
    }

    fn to_solution(&self) -> anyhow::Result<solution::ActiveModel> {
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
