use crate::config::openai::OpenAIConfig;
use anyhow::Context as _;
use dtiku_base::model::{schedule_task, ScheduleTask};
use dtiku_paper::model::{
    question, scraper_solution, ExamCategory, Material, PaperQuestion, Question, Solution,
};
use itertools::Itertools as _;
use openai_api_rs::v1::chat_completion::{self, chat_completion::ChatCompletionRequest, Content};
use reqwest_scraper::ScraperResponse;
use sea_orm::{ActiveValue::Set, EntityTrait as _};
use search_api::{baidu, bing, sogou, SearchItem};
use serde::Deserialize;
use serde_json::Value;
use spring::{plugin::service::Service, tracing};
use spring_opendal::Op;
use spring_sea_orm::DbConn;
use std::fmt::Display;
use url::Url;

#[derive(Debug, Service)]
#[service(prototype)]
pub struct WebSolutionCollectService {
    #[inject(component)]
    db: DbConn,
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
                last_id = qid.max(last_id);
                self.task = self.task.update_context(last_id, &self.db).await?;
            }
        }
    }

    async fn collect_for_question(&self, q: question::Model) -> anyhow::Result<()> {
        let question_id = q.id;

        let question_text = {
            let html = scraper::Html::parse_fragment(q.content.trim());
            html.root_element().text().join("")
        };

        let materials = Material::find_by_qid(&self.db, question_id)
            .await
            .with_context(|| format!("Material::find_by_qid({question_id})"))?;

        let material_text = {
            let material_html = materials.into_iter().map(|m| m.content).join(" ");
            let html = scraper::Html::parse_fragment(material_html.trim());
            html.root_element().text().join("")
        };

        let solutions = Solution::find_by_qid(&self.db, question_id)
            .await
            .with_context(|| format!("Solution::find_by_qid({question_id})"))?;

        let solution_text = {
            let solution_html = solutions.into_iter().map(|s| s.extra.get_html()).join(" ");
            let html = scraper::Html::parse_fragment(solution_html.trim());
            html.root_element().text().join("")
        };

        let result = baidu::search(&question_text).await?;
        self.scraper_web_page_and_save(
            result,
            &question_text,
            &material_text,
            &solution_text,
            question_id,
        )
        .await?;
        let result = sogou::search(&question_text).await?;
        self.scraper_web_page_and_save(
            result,
            &question_text,
            &material_text,
            &solution_text,
            question_id,
        )
        .await?;
        let result = bing::search(&question_text).await?;
        self.scraper_web_page_and_save(
            result,
            &question_text,
            &material_text,
            &solution_text,
            question_id,
        )
        .await?;

        Ok(())
    }

    async fn scraper_web_page_and_save(
        &self,
        search_results: Vec<SearchItem>,
        question: &str,
        material: &str,
        solution: &str,
        question_id: i32,
    ) -> anyhow::Result<()> {
        for SearchItem { url, .. } in search_results {
            let html = self.scraper_web_page(&url).await?;
            let result = self
                .extract_by_llm(&url, &html, question, material, solution)
                .await?;

            scraper_solution::ActiveModel {
                question_id: Set(question_id),
                src_url: Set(url),
                content: Set(result.to_string()),
                ..Default::default()
            }
            .insert_on_conflict(&self.db)
            .await?;
        }
        Ok(())
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
        Ok(html)
    }

    async fn extract_by_llm(
        &self,
        url: &str,
        html: &str,
        question: &str,
        material: &str,
        solution: &str,
    ) -> anyhow::Result<ExtractResult> {
        let uri = Url::parse(url).with_context(|| format!("parse url failed:{url}"))?;
        let model = "deepseek/deepseek-r1-0528:free"; // 目前最强的模型

        let mut html_reader = std::io::Cursor::new(html);
        let readability_page = readability::extractor::extract(&mut html_reader, &uri)
            .context("readability::extractor::extract failed")?;
        let text = &readability_page.text;

        let mut openai = self.openai.clone().build()?;
        let req = ChatCompletionRequest::new(
            model.to_string(),
            vec![
                chat_completion::ChatCompletionMessage {
                    role: chat_completion::MessageRole::system,
                    content: Content::Text(
                        "你是一个信息抽取模型，只根据给定内容抽取信息，严格输出合法JSON，不要包含任何解释、前后缀或markdown代码块。".to_string(),
                    ),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                chat_completion::ChatCompletionMessage {
                    role: chat_completion::MessageRole::user,
                    content: chat_completion::Content::Text(format!(
                        r#"
## text
{text}

## material
{material}

## question
{question}

## solution
{solution}

## 任务
从text中剔除掉material，然后提取出包含solution的关键词密集度最高的一个片段，并且要满足question中的要求，如果存在就返回JSON格式：{{"answer":"原文片段"}}，不存在的话就返回{{"answer":null}}，如果text存在对应的解题思路的片段，在json中添加一个字段：{{"answer":"原文片段","analysis":"解题思路原文片段"}}
"#
                    )),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
        );
        let resp = openai
            .chat_completion(req)
            .await
            .context("chat_completion 调用失败")?;
        let json = resp.choices[0].message.content.clone().unwrap_or_default();
        let json = json.trim_matches('`');

        serde_json::from_str(json).context("parse llm json failed")
    }
}

#[derive(Debug, Deserialize)]
pub struct ExtractResult {
    answer: Option<String>,
    analysis: Option<String>,
}

impl Display for ExtractResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();
        if let Some(answer) = &self.answer {
            parts.push(format!("答案:\n {}", answer));
        }
        if let Some(analysis) = &self.analysis {
            parts.push(format!("解析:\n {}", analysis));
        }

        if parts.is_empty() {
            write!(f, "\n")
        } else {
            write!(f, "{}", parts.join("\n"))
        }
    }
}
