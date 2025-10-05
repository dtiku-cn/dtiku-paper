use super::{JobScheduler, PaperSyncer};
use crate::{
    jobs::{MaterialIdNumber, QuestionIdNumber},
    plugins::embedding::Embedding,
};
use anyhow::anyhow;
use anyhow::Context;
use dtiku_base::model::schedule_task::{self, Progress, TaskInstance};
use dtiku_paper::model::{
    material, paper, paper_material, paper_question,
    question::{self, QuestionExtra},
    solution::{self, MultiChoice, SingleChoice, SolutionExtra, StepByStepAnswer, TrueFalseChoice},
    FromType,
};
use futures::StreamExt as _;
use itertools::Itertools as _;
use scraper::Html;
use sea_orm::{prelude::PgVector, ActiveValue::Set, ConnectionTrait, Statement};
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
                    let exam_id = row.select_from(&self.target_db).await?;

                    sqlx::query("update label set target_id=$1 where id=$2 and from_ty='offcn'")
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
                target_id,
                extra->>'type' as ty,
                extra->>'form' AS form,
                extra->>'stem' as content,
                extra->>'choices' as choices,
                extra->>'answer' as answer,
                extra->>'explain_a' as explain,
                extra->>'explain_a_file' as explain_file,
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

            paper_question::ActiveModel {
                paper_id: Set(paper.id),
                question_id: Set(q_in_db.id),
                sort: Set(*num as i16),
                paper_type: Set(q_in_db.paper_type),
                ..Default::default()
            }
            .insert_on_conflict(&self.target_db)
            .await
            .context("insert paper_question failed")?;
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
    async fn select_from<C: ConnectionTrait>(self, db: &C) -> anyhow::Result<Option<i16>> {
        let stmt = Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            r##"
            select id from exam_category
            where "name" = $1
            "##,
            vec![self.parent_name.clone().into()],
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
    list: Option<Json<Vec<ChapterItem>>>,
    title: String,
    content: Option<String>,
    paper_pattern: Option<i32>,
    id: i64,
    label_id: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterItem {
    pub name: String,
    pub block_id: i64,
    pub done_count: i64,
    pub total_count: i64,
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

#[derive(Debug, Clone, sqlx::FromRow)]
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
    target_id: Option<i32>,
    ty: i16,
    form: i16,
    content: String,
    choices: Option<Json<Vec<Choice>>>,
    answer: Option<Json<Vec<String>>>,
    explain: Option<String>,
    explain_file: Option<Json<Vec<ExplainFile>>>,
    analysis: Option<String>,
    // step_explanation: Option<Json<Vec<String>>>,
    multi_material_id: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
pub struct Choice {
    pub choice: String,
    pub choice_id: i64,
    pub is_correct: i64,
    pub question_id: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
pub struct ExplainFile {
    pub file_url: String,
    pub media_id: String,
    pub file_name: String,
    pub file_type: i64,
    pub use_platform: i64,
}

impl OriginQuestion {
    async fn to_question(&self, model: &Embedding) -> anyhow::Result<question::ActiveModel> {
        let Self {
            form,
            content,
            choices,
            ..
        } = self;

        fn get_options(choices: Option<Json<Vec<Choice>>>) -> Vec<String> {
            choices
                .unwrap_or_default()
                .0
                .into_iter()
                .map(|c| c.choice)
                .collect_vec()
        }
        let extra = match form {
            0 | 1 => QuestionExtra::SingleChoice {
                options: get_options(choices.clone()),
            },
            2 => QuestionExtra::IndefiniteChoice {
                options: get_options(choices.clone()),
            },
            3 => QuestionExtra::TrueFalse,
            4 => QuestionExtra::OpenEndedQA { qa: vec![] },
            5 => QuestionExtra::MultiChoice {
                options: get_options(choices.clone()),
            },
            _ => return Err(anyhow!("异常情况")),
        };

        let txt = {
            let options_string = get_options(choices.clone()).join("\n");
            // scraper::Html 底层用了 tendril::NonAtomic 和 Cell 类型，而这些类型不是线程安全的，所以它 不实现 Send。
            // 用代码块，让html 变量在这里作用域结束，释放掉 Cell 的引用。
            let html = Html::parse_fragment(&format!("{content}\n{options_string}"));
            html.root_element().text().collect::<String>()
        };
        let embedding = model.text_embedding(&txt).await?;
        let mut am = question::ActiveModel {
            content: Set(content.clone()),
            extra: Set(extra),
            embedding: Set(PgVector::from(embedding)),
            ..Default::default()
        };
        if let Some(target_id) = self.target_id {
            am.id = Set(target_id);
        }
        Ok(am)
    }

    fn to_solution(&self) -> anyhow::Result<solution::ActiveModel> {
        let Self {
            form,
            choices,
            answer,
            analysis,
            explain,
            explain_file,
            ..
        } = self;

        fn get_options_answer(choices: Option<Json<Vec<Choice>>>) -> Vec<u8> {
            choices
                .map(|json| {
                    json.0
                        .iter()
                        .enumerate()
                        .filter_map(|(i, c)| {
                            if c.is_correct == 1 {
                                Some(i as u8)
                            } else {
                                None
                            }
                        })
                        .collect()
                })
                .unwrap_or_default()
        }
        let extra = match form {
            0 | 1 => SolutionExtra::SingleChoice(SingleChoice {
                answer: get_options_answer(choices.clone()).remove(0),
                analysis: if let Some(analysis) = analysis {
                    let explain = explain.clone().unwrap_or_default();
                    format!("{analysis}\n{explain}")
                } else {
                    explain.clone().unwrap_or_default()
                },
            }),
            2 => SolutionExtra::IndefiniteChoice(MultiChoice {
                answer: get_options_answer(choices.clone()),
                analysis: if let Some(analysis) = analysis {
                    let explain = explain.clone().unwrap_or_default();
                    format!("{analysis}\n{explain}")
                } else {
                    explain.clone().unwrap_or_default()
                },
            }),
            3 => SolutionExtra::TrueFalse(TrueFalseChoice {
                answer: true,
                analysis: if let Some(analysis) = analysis {
                    let explain = explain.clone().unwrap_or_default();
                    format!("{analysis}\n{explain}")
                } else {
                    explain.clone().unwrap_or_default()
                },
            }),
            4 => SolutionExtra::OpenEndedQA(StepByStepAnswer {
                solution: Some(answer.clone().unwrap().0.join("\n")),
                analysis: vec![],
            }),
            5 => SolutionExtra::MultiChoice(MultiChoice {
                answer: get_options_answer(choices.clone()),
                analysis: if let Some(analysis) = analysis {
                    let explain = explain.clone().unwrap_or_default();
                    format!("{analysis}\n{explain}")
                } else {
                    explain.clone().unwrap_or_default()
                },
            }),
            _ => return Err(anyhow!("异常情况")),
        };

        Ok(solution::ActiveModel {
            extra: Set(extra),
            ..Default::default()
        })
    }
}
