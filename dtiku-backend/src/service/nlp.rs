use crate::utils::hnsw::HNSWIndex;
use crate::utils::regex as regex_util;
use crate::{plugins::embedding::Embedding, utils::hnsw::LabeledSentence};
use anyhow::Context;
use dtiku_paper::model::{Question, Solution};
use sea_orm::EntityTrait;
use spring::plugin::service::Service;
use spring_sea_orm::DbConn;

#[derive(Debug, Clone, Service)]
pub struct NLPService {
    #[inject(component)]
    pub embedding: Embedding,
    #[inject(component)]
    pub db: DbConn,
}

impl NLPService {
    pub async fn build_hnsw_index_for_question(
        &self,
        question_id: i32,
    ) -> anyhow::Result<Option<HNSWIndex>> {
        let question = Question::find_by_id(question_id)
            .one(&self.db)
            .await
            .with_context(|| format!("Question::find_by_id({question_id})"))?;

        let question = if let Some(question) = question {
            question
        } else {
            return Ok(None);
        };

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

        // 收集 solution 的句子
        for solution in solutions {
            let solution_html = solution.extra.get_html();
            for sentence in regex_util::split_sentences(&solution_html) {
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

        let hnsw = HNSWIndex::build(&all_sentences);

        Ok(Some(hnsw))
    }
}
