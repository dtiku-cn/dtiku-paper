use crate::config::openai::OpenAIConfig;
use crate::plugins::embedding::Embedding;
use crate::utils::hnsw::{HNSWIndex, IdAndEmbedding};
use crate::utils::regex as regex_util;
use anyhow::Context;
use dtiku_paper::model::{Material, Question, Solution};
use itertools::Itertools;
use scraper::Html;
use sea_orm::EntityTrait;
use spring::plugin::service::Service;
use spring_sea_orm::DbConn;
use std::collections::HashMap;

#[derive(Debug, Clone, Service)]
pub struct NLPService {
    #[inject(component)]
    pub embedding: Embedding,
    #[inject(config)]
    pub openai: OpenAIConfig,
    #[inject(component)]
    pub db: DbConn,
}

#[derive(Debug, Clone)]
pub struct LabeledSentence {
    pub id: usize,
    pub label: String, // "question" or "solution"
    pub outer_id: i32,
    pub text: String,
    pub embedding: Vec<f32>,
}

impl IdAndEmbedding for LabeledSentence {
    fn id(&self) -> usize {
        self.id
    }

    fn embedding(&self) -> &[f32] {
        &self.embedding
    }
}

#[derive(Debug, Clone)]
pub struct SolutionKeyword {
    pub id: usize,
    pub word: String,
    pub embedding: Vec<f32>,
}

impl IdAndEmbedding for SolutionKeyword {
    fn id(&self) -> usize {
        self.id
    }

    fn embedding(&self) -> &[f32] {
        &self.embedding
    }
}

impl NLPService {
    pub async fn build_hnsw_index_for_question(
        &self,
        question_id: i32,
    ) -> anyhow::Result<Option<HNSWIndex<LabeledSentence>>> {
        let openai_client = self.openai.clone().build()?;
        let question = Question::find_by_id(question_id)
            .one(&self.db)
            .await
            .with_context(|| format!("Question::find_by_id({question_id})"))?;

        let question = if let Some(question) = question {
            question
        } else {
            return Ok(None);
        };

        let materials = Material::find_by_qid(&self.db, question_id)
            .await
            .with_context(|| format!("Material::find_by_qid({question_id})"))?;

        let solutions = Solution::find_by_qid(&self.db, question_id)
            .await
            .with_context(|| format!("Solution::find_by_qid({question_id})"))?;

        let mut all_sentences = vec![];
        let mut sentence_meta = vec![]; // 存储每个句子的元信息：label 和 outer_id
        let mut id = 0;

        // 收集 question 的句子
        for sentence in regex_util::split_sentences(&question.content) {
            sentence_meta.push(("question", question_id, sentence.to_string()));
        }

        // 收集 material 的句子
        for material in materials {
            let material_text = {
                let html = Html::parse_fragment(&material.content);
                html.root_element().text().join(" ")
            };
            for sentence in regex_util::split_sentences(&material_text) {
                sentence_meta.push(("material", material.id, sentence.to_string()));
            }
        }

        let mut all_solution_text = String::new();
        // 收集 solution 的句子
        for solution in solutions {
            let solution_text = {
                let html = Html::parse_fragment(&solution.extra.get_html());
                html.root_element().text().join(" ")
            };
            all_solution_text.push_str(&solution_text);
            for sentence in regex_util::split_sentences(&solution_text) {
                sentence_meta.push(("solution", solution.id, sentence.to_string()));
            }
        }

        // 提取句子文本用于 batch_embedding
        let texts: Vec<String> = sentence_meta
            .iter()
            .map(|(_, _, text)| text.clone())
            .collect();
        let embeddings = self.embedding.batch_text_embedding(&texts).await?;

        // 构建 LabeledSentence 列表
        for ((label, outer_id, text), embedding) in sentence_meta.into_iter().zip(embeddings) {
            all_sentences.push(LabeledSentence {
                id,
                label: label.into(),
                outer_id,
                text,
                embedding,
            });
            id += 1;
        }

        let sentence_hnsw = HNSWIndex::build(&all_sentences);

        Ok(Some(sentence_hnsw))
    }

    pub async fn build_hnsw_index_for_label_text(
        &self,
        label_text: HashMap<String, String>,
    ) -> anyhow::Result<HNSWIndex<LabeledSentence>> {
        let mut all_sentences = vec![];
        let mut sentence_meta = vec![]; // 存储每个句子的元信息：label 和 outer_id
        let mut id = 0;

        // 收集 question 的句子
        for (label, text) in &label_text {
            for sentence in regex_util::split_sentences(&text) {
                sentence_meta.push((label.as_str(), sentence.to_string()));
            }
        }

        // 提取句子文本用于 batch_embedding
        let texts: Vec<String> = sentence_meta.iter().map(|(_, text)| text.clone()).collect();
        let embeddings = self.embedding.batch_text_embedding(&texts).await?;

        // 构建 LabeledSentence 列表
        for ((label, text), embedding) in sentence_meta.into_iter().zip(embeddings) {
            all_sentences.push(LabeledSentence {
                id,
                label: label.into(),
                outer_id: 0, // outer_id 在这里不使用
                text,
                embedding,
            });
            id += 1;
        }

        let sentence_hnsw = HNSWIndex::build(&all_sentences);

        Ok(sentence_hnsw)
    }
}
