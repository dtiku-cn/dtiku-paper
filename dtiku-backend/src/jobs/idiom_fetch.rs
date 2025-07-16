use std::time::Duration;

use anyhow::Context as _;
use dtiku_base::model::schedule_task;
use dtiku_paper::model::{paper, paper_question, ExamCategory, Paper, PaperQuestion, Question};
use dtiku_stats::model::{
    idiom::{self, IdiomExplain as IdiomExplainModel},
    idiom_ref,
    sea_orm_active_enums::IdiomType,
    IdiomRef,
};
use itertools::Itertools;
use reqwest;
use reqwest_scraper::{FromCssSelector, ScraperResponse};
use sea_orm::{sea_query::ExprTrait, ActiveValue::Set, EntityTrait, Iterable};
use serde_json::Value;
use spring::{plugin::service::Service, tracing};
use spring_sea_orm::DbConn;

#[derive(Debug, FromCssSelector)]
pub struct IdiomExplain {
    #[selector(path = "#shiyiDiv", inner_html)]
    shiyi: Option<String>,

    #[selector(path = "#shiyidetailDiv", inner_html)]
    shiyidetail: Option<String>,

    #[selector(path = "#liju ul.item-list", html)]
    liju: Option<String>,

    #[selector(path = "#jyc ul.words-list>li a.text-default", text)]
    jyc: Vec<String>,

    #[selector(path = "#fyc ul.words-list>li a.text-default", text)]
    fyc: Vec<String>,
}

impl IdiomExplain {
    pub async fn fetch(idiom: &str) -> anyhow::Result<Self> {
        let html = reqwest::get(format!("https://hanyu.sogou.com/result?query={idiom}"))
            .await?
            .css_selector()
            .await?;

        Ok(Self::from_html(html)?)
    }
}

impl Into<IdiomExplainModel> for IdiomExplain {
    fn into(self) -> IdiomExplainModel {
        IdiomExplainModel {
            shiyi: self.shiyi.unwrap_or_default(),
            shiyidetail: self.shiyidetail.unwrap_or_default(),
            liju: self.liju.unwrap_or_default(),
            jyc: self.jyc,
            fyc: self.fyc,
        }
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
    }

    pub async fn stats_for_papers(&mut self, paper_type: i16) -> anyhow::Result<()> {
        let mut last_id = match &self.task.context {
            Value::Number(last_id) => last_id.as_i64().unwrap_or_default() as i32,
            _ => 0,
        };
        loop {
            let papers = Paper::find_by_paper_type_and_id_gt(&self.db, paper_type, last_id)
                .await
                .expect("find_by_paper_type_and_id_gt failed");

            if papers.is_empty() {
                return Ok(());
            }

            for paper in papers {
                let paper_id = paper.id;
                if let Err(e) = self.stats_for_paper_detail(paper).await {
                    tracing::error!("stats_for_paper_detail({}) error: {:?}", paper_id, e);
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

        let qids = PaperQuestion::find_question_ids_by_paper_id_and_sort_between(
            &self.db, paper.id, start, end,
        )
        .await?;

        if qids.is_empty() {
            tracing::warn!(
                "paper_id: {}, no questions found in range {}-{}",
                paper.id,
                start,
                end
            );
            return Ok(());
        }
        let mut idiom_count = 0;
        let mut idiom_refs = Vec::<idiom_ref::ActiveModel>::new();

        let questions = Question::find_by_ids(&self.db, qids).await?;

        for q in questions {
            for ty in IdiomType::iter() {
                let idioms = ty
                    .regex()
                    .captures_iter(&q.content)
                    .map(|cap| cap.get(0).unwrap().as_str().trim())
                    .collect_vec();

                for idiom in idioms {
                    let idiom = idiom::ActiveModel {
                        text: Set(idiom.to_string()),
                        ty: Set(ty),
                        content: Set(IdiomExplain::fetch(idiom).await?.into()),
                        ..Default::default()
                    }
                    .insert_on_conflict(&self.db)
                    .await
                    .context("insert idiom failed")?;
                    idiom_count += 1;

                    idiom_refs.push(idiom_ref::ActiveModel {
                        ty: Set(idiom.ty),
                        idiom_id: Set(idiom.id),
                        question_id: Set(q.id),
                        paper_id: Set(paper.id),
                        label_id: Set(paper.label_id),
                        exam_id: Set(paper.exam_id),
                        paper_type: Set(paper.paper_type),
                        ..Default::default()
                    });
                }

                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        }
        IdiomRef::insert_many(idiom_refs)
            .exec(&self.db)
            .await
            .context("insert idiom_refs failed")?;

        tracing::info!("paper_id: {}, idiom_count: {}", paper.id, idiom_count);

        Ok(())
    }
}
