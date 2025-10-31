use anyhow::Context as _;
use dtiku_base::model::{schedule_task, ScheduleTask};
use dtiku_paper::model::{
    paper, question::QuestionExtra, ExamCategory, Paper, PaperQuestion, Question,
};
use dtiku_stats::model::{
    idiom::{self, IdiomExplainEntry},
    idiom_ref,
    sea_orm_active_enums::IdiomType,
    Idiom,
};
use itertools::Itertools;
use reqwest;
use sea_orm::{ActiveValue::Set, EntityTrait, Iterable};
use serde::Deserialize;
use serde_json::Value;
use spring::{plugin::service::Service, tracing};
use spring_sea_orm::DbConn;
use std::time::Duration;

////////////////////////////////baidu
///
#[derive(Debug, Deserialize)]
pub struct BaiduApiResponse {
    pub errno: i32,
    pub errmsg: String,
    pub data: IdiomExplainEntry,
}

impl BaiduApiResponse {
    pub async fn fetch(idiom: &str) -> anyhow::Result<Self> {
        let term_detail = reqwest::get(format!("https://hanyuapp.baidu.com/dictapp/swan/termdetail?wd={idiom}&client=pc&source_tag=2&smp_names=termBrand2,poem1&lesson_from=xiaodu"))
            .await?
            .json()
            .await?;

        Ok(term_detail)
    }
}

#[derive(Debug, Service)]
#[service(prototype)]
pub struct IdiomStatsService {
    #[inject(component)]
    db: DbConn,
    task: schedule_task::Model,
}

impl IdiomStatsService {
    pub async fn start(&mut self) {
        let paper_type = ExamCategory::find_category_id_by_path(&self.db, "gwy/xingce")
            .await
            .expect("gwy/xingce category found failed")
            .expect("gwy/xingce category id not found");

        self.stats_for_papers(paper_type)
            .await
            .expect(&format!("stats idiom for paper_type#{paper_type} failed"));

        let _ = ScheduleTask::update(schedule_task::ActiveModel {
            id: Set(self.task.id),
            version: Set(self.task.version + 1),
            active: Set(false),
            ..Default::default()
        })
        .exec(&self.db)
        .await
        .is_err_and(|e| {
            tracing::error!("update task error: {:?}", e);
            false
        });
    }

    pub async fn stats_for_papers(&mut self, paper_type: i16) -> anyhow::Result<()> {
        let mut last_id = match &self.task.context {
            Value::Number(last_id) => last_id.as_i64().unwrap_or_default() as i32,
            _ => 0,
        };
        tracing::warn!("stats_for_papers({paper_type}) started");
        loop {
            let papers = Paper::find_by_paper_type_and_id_gt(&self.db, paper_type, last_id)
                .await
                .expect("find_by_paper_type_and_id_gt failed");

            if papers.is_empty() {
                tracing::warn!("stats_for_papers({paper_type}) finished");
                return Ok(());
            }

            for paper in papers {
                let paper_id = paper.id;
                if let Err(e) = self.stats_for_paper_detail(paper).await {
                    tracing::error!("stats_for_paper_detail({paper_id}) error: {e:?}");
                }
                last_id = paper_id;
                self.task = self.task.update_context(last_id, &self.db).await?;
            }
        }
    }

    pub async fn stats_for_paper_detail(&mut self, paper: paper::Model) -> anyhow::Result<()> {
        let (start, end) = if let Some(range) = paper.extra.compute_question_range("言语理解") {
            range
        } else {
            return Ok(());
        };

        let paper_id = paper.id;

        let qid_sort =
            PaperQuestion::find_by_paper_id_and_sort_between(&self.db, paper_id, start, end)
                .await?;

        if qid_sort.is_empty() {
            tracing::warn!("paper_id: {paper_id}, no questions found in range {start}-{end}");
            return Ok(());
        }
        let mut idiom_count = 0;
        let mut idiom_ref_count = 0;

        let qids = qid_sort.keys().cloned().collect();
        let questions = Question::find_by_ids(&self.db, qids).await?;

        for q in questions {
            for ty in IdiomType::iter() {
                match &q.extra {
                    QuestionExtra::SingleChoice { options }
                    | QuestionExtra::MultiChoice { options } => {
                        let options = options.join(" \t");
                        let idioms = ty
                            .regex()
                            .captures_iter(&options)
                            .map(|res| {
                                let cap = res?; // Result<Captures>
                                Ok(cap.get(0).map(|m| m.as_str().trim()))
                            })
                            .collect::<Result<Vec<_>, fancy_regex::Error>>()?
                            .into_iter()
                            .flatten()
                            .collect_vec();

                        for idiom in idioms {
                            let idiom_in_db = if let Some(idiom_in_db) =
                                Idiom::find_by_text(&self.db, idiom).await?
                            {
                                idiom_in_db
                            } else {
                                let resp = BaiduApiResponse::fetch(idiom).await;
                                if let Err(e) = resp {
                                    tracing::warn!("拉取【{idiom}】百度字典失败:{e}");
                                    continue;
                                } else {
                                    tokio::time::sleep(Duration::from_secs(1)).await;
                                }
                                let explain = resp.unwrap().data;
                                let basic_explain = (&explain).into();
                                let saved_idiom = idiom::ActiveModel {
                                    text: Set(idiom.to_string()),
                                    ty: Set(ty),
                                    explain: Set(basic_explain),
                                    content: Set(explain.into()),
                                    ..Default::default()
                                }
                                .insert_on_conflict(&self.db)
                                .await
                                .context("insert idiom failed")?;
                                idiom_count += 1;
                                saved_idiom
                            };

                            idiom_ref::ActiveModel {
                                ty: Set(idiom_in_db.ty),
                                idiom_id: Set(idiom_in_db.id),
                                question_id: Set(q.id),
                                paper_id: Set(paper.id),
                                sort: Set(*qid_sort.get(&q.id).expect("qid sort not found")),
                                label_id: Set(paper.label_id),
                                exam_id: Set(paper.exam_id),
                                paper_type: Set(paper.paper_type),
                                ..Default::default()
                            }
                            .insert_on_conflict(&self.db)
                            .await
                            .context("insert idiom failed")?;
                            idiom_ref_count += 1;
                        }
                    }
                    _ => {}
                }
            }
        }
        let sleep_secondes = rand::random::<u64>() % 10;
        tracing::info!(
            "paper_id: {paper_id}, idiom_count: {idiom_count}, idiom_ref_count: {idiom_ref_count}, will sleep {sleep_secondes}s"
        );
        tokio::time::sleep(Duration::from_secs(sleep_secondes)).await;

        Ok(())
    }
}
