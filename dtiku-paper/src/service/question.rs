use crate::{
    domain::question::QuestionSearch,
    model::{self, question, Paper, PaperQuestion, Question},
    query::question::PaperQuestionQuery,
};
use itertools::Itertools;
use sea_orm::DbConn;
use spring::plugin::service::Service;

#[derive(Clone, Service)]
pub struct QuestionService {
    #[inject(component)]
    db: DbConn,
}

impl QuestionService {
    pub async fn search_question(
        &self,
        query: &QuestionSearch,
    ) -> anyhow::Result<Vec<question::QuestionWithPaper>> {
        Question::search_question(&self.db, query).await
    }

    pub async fn search_question_by_section(
        &self,
        query: &PaperQuestionQuery,
    ) -> anyhow::Result<(Vec<question::Model>, Vec<model::paper::Model>)> {
        if query.paper_ids.is_empty() {
            return Ok((vec![], vec![]));
        }
        let ps = Paper::find_by_ids(&self.db, query.paper_ids.clone()).await?;
        let pqs = PaperQuestion::find_by_query(&self.db, query).await?;
        let qids = pqs.iter().map(|pq| pq.question_id).collect_vec();
        let qs = Question::find_by_ids(&self.db, qids).await?;
        Ok((qs, ps))
    }

    pub async fn full_question_by_id(&self, id: i32) -> anyhow::Result<()> {
        todo!()
    }
}
