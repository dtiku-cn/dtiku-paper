use sea_orm::DbConn;
use spring::plugin::service::Service;

use crate::{
    model::{question, PaperQuestion, Question},
    query::question::PaperQuestionQuery,
};

#[derive(Clone, Service)]
pub struct QuestionService {
    #[inject(component)]
    db: DbConn,
}

impl QuestionService {
    pub async fn find_question_by_query(
        &self,
        query: &PaperQuestionQuery,
    ) -> anyhow::Result<Vec<question::Model>> {
        let qids = PaperQuestion::find_by_query(&self.db, query).await?;
        // Question::find_by_ids(&self.db, qids).await
        todo!()
    }
}
