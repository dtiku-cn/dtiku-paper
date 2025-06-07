use crate::{
    domain::question::QuestionSearch,
    model::{
        self, paper_question,
        question::{self, PaperWithNum, QuestionWithPaper},
        Material, Paper, PaperQuestion, Question, QuestionMaterial, Solution,
    },
    query::question::PaperQuestionQuery,
};
use anyhow::Context;
use itertools::Itertools;
use sea_orm::{DbConn, EntityTrait};
use spring::plugin::service::Service;
use std::collections::HashMap;

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

    pub async fn full_question_by_id(&self, id: i32) -> anyhow::Result<Option<QuestionWithPaper>> {
        let q = Question::find_by_id(id).one(&self.db).await?;
        Ok(match q {
            None => None,
            Some(q) => {
                let mids = QuestionMaterial::find_by_qid(&self.db, q.id)
                    .await
                    .context("find question material failed")?;

                let ms = Material::find_by_ids(&self.db, mids).await?;
                let ss = Solution::find_by_qid(&self.db, q.id).await?;

                let pq = PaperQuestion::find_by_question_id(&self.db, q.id)
                    .await
                    .context("find question paper failed")?;

                let pid_map: HashMap<i32, paper_question::Model> =
                    pq.into_iter().map(|p| (p.paper_id, p)).collect();
                let pids = pid_map.keys().cloned().collect_vec();
                let papers = Paper::find_by_ids(&self.db, pids).await?;
                let papers = papers
                    .iter()
                    .map(|p| {
                        PaperWithNum::new(p, pid_map.get(&p.id).map(|p| p.sort).unwrap_or_default())
                    })
                    .collect_vec();

                Some(QuestionWithPaper::new(q, papers, Some(ss), Some(ms)))
            }
        })
    }
}
