use crate::utils::regex as regex_util;
use crate::{
    config::openai::OpenAIConfig,
    plugins::embedding::Embedding,
    utils::hnsw::{HNSWIndex, LabeledSentence},
};
use anyhow::Context as _;
use dtiku_base::model::{schedule_task, ScheduleTask};
use dtiku_paper::model::{question, ExamCategory, PaperQuestion, Question, Solution};
use itertools::Itertools as _;
use reqwest_scraper::ScraperResponse;
use sea_orm::{ActiveValue::Set, EntityTrait as _};
use search_api::{baidu, bing, sogou, SearchItem};
use serde::Deserialize;
use serde_json::Value;
use spring::{plugin::service::Service, tracing};
use spring_opendal::Op;
use spring_sea_orm::DbConn;
use url::Url;

#[derive(Debug, Service)]
#[service(prototype)]
pub struct WebSolutionCollectService {
    #[inject(component)]
    db: DbConn,
    #[inject(component)]
    embedding: Embedding,
    #[inject(config)]
    openai: OpenAIConfig,
    #[inject(component)]
    op: Op,
    task: schedule_task::Model,
}

impl WebSolutionCollectService {
    pub async fn start(&mut self) {
        let paper_type = ExamCategory::find_category_id_by_path(&self.db, "gwy/shenlun")
            .await
            .expect("gwy/shenlun category found failed")
            .expect("gwy/shenlun category id not found");

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

        let text = {
            let html = scraper::Html::parse_fragment(content);
            html.root_element().text().join("")
        };

        let hnsw_index = self.build_hnsw_index_for_question(&q).await?;

        let result = baidu::search(&text).await?;
        self.scraper_web_page_and_save(&hnsw_index, result).await?;
        let result = sogou::search(&text).await?;
        self.scraper_web_page_and_save(&hnsw_index, result).await?;
        let result = bing::search(&text).await?;
        self.scraper_web_page_and_save(&hnsw_index, result).await?;

        Ok(())
    }

    async fn scraper_web_page_and_save(
        &self,
        hnsw_index: &HNSWIndex,
        search_results: Vec<SearchItem>,
    ) -> anyhow::Result<()> {
        for SearchItem { url, .. } in search_results {
            let html = self.scraper_web_page(&url).await?;
            let mut semantic_tree = {
                let dom = scraper::Html::parse_fragment(&html);
                self.build_semantic_subtree(dom.root_element())
            };
            self.compute_semantic_tree(hnsw_index, &mut semantic_tree)
                .await?;
        }
        Ok(())
    }

    pub async fn build_hnsw_index_for_question(
        &self,
        question: &question::Model,
    ) -> anyhow::Result<HNSWIndex> {
        let question_id = question.id;
        let solutions = Solution::find_by_qid(&self.db, question_id)
            .await
            .with_context(|| format!("Solution::find_by_qid({question_id})"))?;

        let mut all_sentences = vec![];
        let mut id = 0;
        for sentence in regex_util::split_sentences(&question.content) {
            all_sentences.push(LabeledSentence {
                id,
                label: "question".into(),
                outer_id: question_id,
                text: sentence.to_string(),
                embedding: self.embedding.text_embedding(sentence).await?,
            });
            id += 1;
        }
        for solution in solutions {
            let solution_html = solution.extra.get_html();
            for sentence in regex_util::split_sentences(&solution_html) {
                all_sentences.push(LabeledSentence {
                    id,
                    label: "solution".into(),
                    outer_id: solution.id,
                    text: sentence.to_string(),
                    embedding: self.embedding.text_embedding(sentence).await?,
                });
                id += 1;
            }
        }

        Ok(HNSWIndex::build(&all_sentences))
    }

    async fn scraper_web_page(&self, url: &str) -> anyhow::Result<String> {
        let uri = Url::parse(url).with_context(|| format!("parse url failed:{url}"))?;
        let md5 = md5::compute(url);
        let domain = uri.domain().unwrap_or("null");
        let dir = format!("{domain}/{md5:x}");
        let html_file = format!("{dir}/{url}");
        let html = if self
            .op
            .exists(&html_file)
            .await
            .with_context(|| format!("op.exists({html_file}) failed"))?
        {
            let r = self
                .op
                .read(&html_file)
                .await
                .with_context(|| format!("op.read({html_file}) failed"))?;

            String::from_utf8(r.to_vec()).context("parse to string failed")?
        } else {
            let resp = reqwest::get(url).await?;
            let html = resp.html().await?;
            self.op.write(&html_file, html.clone()).await?;
            html
        };

        let mut html_reader = std::io::Cursor::new(html.as_str());
        let readability_page = readability::extractor::extract(&mut html_reader, &uri)
            .context("readability::extractor::extract failed")?;
        Ok(readability_page.content)
    }

    async fn compute_semantic_tree(
        &self,
        hnsw_index: &HNSWIndex,
        semantic_tree: &mut SemanticNode,
    ) -> anyhow::Result<()> {
        if !semantic_tree.text.is_empty() {
            let embedding = self.embedding.text_embedding(&semantic_tree.text).await?;
            let result = hnsw_index.search(&embedding, 1);
            semantic_tree.embedding = Some(embedding);
            if !result.is_empty() {
                semantic_tree.label = Some(result[0].0.label.clone());
                semantic_tree.similarity = Some(result[0].1);
            }
        }
        Ok(())
    }

    fn build_semantic_subtree(&self, node: scraper::ElementRef) -> SemanticNode {
        // 递归处理子节点
        let mut children = Vec::new();
        for child in node.children() {
            if let Some(element) = child.value().as_element() {
                if [
                    "p",
                    "div",
                    "h1",
                    "h2",
                    "h3",
                    "h4",
                    "h5",
                    "h6",
                    "ul",
                    "ol",
                    "li",
                    "dl",
                    "dt",
                    "dd",
                    "blockquote",
                    "article",
                    "section",
                    "main",
                    "details",
                    "summary",
                ]
                .contains(&element.name())
                {
                    let child_node =
                        self.build_semantic_subtree(scraper::ElementRef::wrap(child).unwrap());
                    children.push(child_node);
                }
            }
        }

        SemanticNode {
            children,
            text: node.text().join(" "),
            html: node.html(),
            label: None,
            embedding: None,
            similarity: None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct QuestionSolution {
    question: String,
    solution: String,
}

#[derive(Debug, Clone)]
pub struct SemanticNode {
    pub children: Vec<SemanticNode>,
    pub html: String,
    pub text: String,
    pub label: Option<String>,
    pub embedding: Option<Vec<f32>>,
    pub similarity: Option<f32>,
}
