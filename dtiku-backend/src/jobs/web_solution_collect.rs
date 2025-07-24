use super::search::{baidu, bing, sogou, SearchItem};
use anyhow::Context as _;
use dtiku_base::model::{schedule_task, ScheduleTask};
use dtiku_paper::model::{question, ExamCategory, PaperQuestion, Question};
use itertools::Itertools as _;
use reqwest_scraper::ScraperResponse;
use sea_orm::{ActiveValue::Set, EntityTrait as _};
use serde_json::Value;
use spring::{plugin::service::Service, tracing};
use spring_sea_orm::DbConn;

#[derive(Debug, Service)]
#[service(prototype)]
pub struct WebSolutionCollectService {
    #[inject(component)]
    db: DbConn,
    task: schedule_task::Model,
}

impl WebSolutionCollectService {
    pub async fn start(&mut self) {
        let paper_type = ExamCategory::find_category_id_by_path(&self.db, "gwy/xingce")
            .await
            .expect("gwy/xingce category found failed")
            .expect("gwy/xingce category id not found");

        self.collect_for_papers(paper_type).await.expect(&format!(
            "collect solution for papers for paper_type#{paper_type} failed"
        ));

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

    pub async fn collect_for_papers(&mut self, paper_type: i16) -> anyhow::Result<()> {
        let mut last_id = match &self.task.context {
            Value::Number(last_id) => last_id.as_i64().unwrap_or_default() as i32,
            _ => 0,
        };
        tracing::warn!("collect_for_papers({paper_type}) started");

        loop {
            let qids =
                PaperQuestion::find_by_paper_type_and_qid_gt(&self.db, paper_type, last_id).await?;
            if qids.is_empty() {
                tracing::warn!("collect_for_papers({paper_type}) finished");
                return Ok(());
            }
            let questions = Question::find_by_ids(&self.db, qids).await?;
            for q in questions {
                let qid = q.id;
                if let Err(e) = self.collect_for_question(q).await {
                    tracing::error!("collect_for_question({qid}) error: {e:?}");
                }
                last_id = qid;
                self.task = self.task.update_context(last_id, &self.db).await?;
            }
        }
    }

    async fn collect_for_question(&self, q: question::Model) -> anyhow::Result<()> {
        let content = q.content.trim();

        let html = scraper::Html::parse_fragment(content);
        let text = html.root_element().text().join("");

        let result = baidu::search(&text).await?;
        self.scraper_web_page(result).await;
        let result = sogou::search(&text).await?;
        self.scraper_web_page(result).await;
        let result = bing::search(&text).await?;
        self.scraper_web_page(result).await;

        Ok(())
    }

    async fn scraper_web_page(&self, result: Vec<SearchItem>) -> anyhow::Result<()> {
        for SearchItem { url, desc, title } in result {
            let resp = reqwest::get(&url).await?;
            let html = resp.html().await?;
            let url = url::Url::parse(&url).with_context(|| format!("parse url failed:{url}"))?;

            let mut html_reader = std::io::Cursor::new(html.clone());
            let readability_page = readability::extractor::extract(&mut html_reader, &url)
                .context("readability::extractor::extract failed")?;
            let text = &readability_page.text;
        }
        Ok(())
    }
}
