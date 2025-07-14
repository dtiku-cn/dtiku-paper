use dtiku_base::model::schedule_task;
use dtiku_paper::model::{paper, paper_question, ExamCategory, Paper, PaperQuestion, Question};
use dtiku_stats::model::{idiom, idiom_ref};
use reqwest;
use reqwest_scraper::{FromCssSelector, ScraperResponse};
use sea_orm::sea_query::ExprTrait;
use spring::{plugin::service::Service, tracing};
use spring_sea_orm::DbConn;

#[derive(Debug, FromCssSelector)]
pub struct Idiom {
    #[selector(path = "#main div.words-details h4>span", default = "<uname>", text)]
    idiom: String,

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

impl Idiom {
    pub async fn fetch_explain(idiom: &str) -> anyhow::Result<Self> {
        let html = reqwest::get(format!("https://hanyu.sogou.com/result?query={idiom}"))
            .await?
            .css_selector()
            .await?;

        Ok(Self::from_html(html)?)
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

        self.stats_for_papers(paper_type).await.expect("");
    }

    pub async fn stats_for_papers(&mut self, paper_type: i16) -> anyhow::Result<()> {
        let mut last_id = 0;
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
            }
        }
    }

    pub async fn stats_for_paper_detail(&mut self, paper: paper::Model) -> anyhow::Result<()> {
        let (start, end) = paper.extra.compute_question_range("言语理解");

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
        let mut idioms = Vec::<idiom::ActiveModel>::new();
        let mut idiom_refs = Vec::<idiom_ref::ActiveModel>::new();

        let questions = Question::find_by_ids(&self.db, qids).await?;

        for q in questions {}

        tracing::info!("paper_id: {}, idiom_count: {}", paper.id, idioms.len());

        Ok(())
    }
}
