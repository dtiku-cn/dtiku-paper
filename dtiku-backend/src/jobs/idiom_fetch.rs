use std::time::Duration;

use anyhow::Context as _;
use dtiku_base::model::schedule_task;
use dtiku_paper::model::{
    paper, question::QuestionExtra, ExamCategory, Paper, PaperQuestion, Question,
};
use dtiku_stats::model::{
    idiom::{self, IdiomExplain as IdiomExplainModel},
    idiom_ref,
    sea_orm_active_enums::IdiomType,
    Idiom,
};
use itertools::Itertools;
use reqwest;
use reqwest_scraper::{FromCssSelector, ScraperResponse};
use sea_orm::{ActiveValue::Set, Iterable};
use serde_json::Value;
use spring::{plugin::service::Service, tracing};
use spring_sea_orm::DbConn;

#[derive(Debug, FromCssSelector)]
pub struct IdiomExplain {
    #[selector(
        path = "#main div.words-details h4>span",
        default = "<undefined>",
        text
    )]
    idiom: String,

    #[selector(path = "#shiyiDiv", text)]
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
            shiyidetail: self.shiyidetail.unwrap_or_default().replace(" ", ""),
            liju: self.liju.unwrap_or_default().replace(" ", ""),
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
                            if Idiom::exists_by_text(&self.db, idiom).await? {
                                continue;
                            }
                            let explain = IdiomExplain::fetch(idiom).await?;
                            if explain.idiom == "<undefined>" {
                                continue;
                            } else {
                                tokio::time::sleep(Duration::from_secs(1)).await;
                            }
                            let main_explain = explain.shiyi.clone();
                            let saved_idiom = idiom::ActiveModel {
                                text: Set(idiom.to_string()),
                                ty: Set(ty),
                                explain: Set(main_explain.unwrap_or_default()),
                                content: Set(explain.into()),
                                ..Default::default()
                            }
                            .insert_on_conflict(&self.db)
                            .await
                            .context("insert idiom failed")?;
                            idiom_count += 1;

                            idiom_ref::ActiveModel {
                                ty: Set(saved_idiom.ty),
                                idiom_id: Set(saved_idiom.id),
                                question_id: Set(q.id),
                                paper_id: Set(paper.id),
                                label_id: Set(paper.label_id),
                                exam_id: Set(paper.exam_id),
                                paper_type: Set(paper.paper_type),
                                ..Default::default()
                            }
                            .insert_on_conflict(&self.db)
                            .await
                            .context("insert idiom failed")?;
                        }
                    }
                    _ => {}
                }
            }
        }

        tracing::info!("paper_id: {}, idiom_count: {}", paper.id, idiom_count);

        Ok(())
    }
}
