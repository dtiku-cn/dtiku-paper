use crate::{
    domain::question::QuestionSearch,
    model::{question, PaperQuestion, Question},
    query::question::PaperQuestionQuery,
};
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

    pub async fn find_question_by_query(
        &self,
        query: &PaperQuestionQuery,
    ) -> anyhow::Result<Vec<question::Model>> {
        let _qids = PaperQuestion::find_by_query(&self.db, query).await?;
        // Question::find_by_ids(&self.db, qids).await
        todo!()
    }
}
