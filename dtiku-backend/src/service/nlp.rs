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
        let mut id = 0;
        for s in regex_util::split_sentences(&question.content) {
            all_sentences.push(LabeledSentence {
                id,
                label: "question".into(),
                outer_id: question_id,
                text: s.clone(),
                embedding: self.embedding.text_embedding(&s).await?, // 假设你有 embed(text) -> Vec<f32>
            });
            id += 1;
        }
        for solution in solutions {
            for s in regex_util::split_sentences(&solution.extra.get_html()) {
                all_sentences.push(LabeledSentence {
                    id,
                    label: "solution".into(),
                    outer_id: solution.id,
                    text: s.clone(),
                    embedding: self.embedding.text_embedding(&s).await?,
                });
                id += 1;
            }
        }
        let hnsw = HNSWIndex::build(&all_sentences);

        Ok(Some(hnsw))
    }
}
