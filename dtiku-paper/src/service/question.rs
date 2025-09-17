use crate::{
    domain::question::QuestionSearch,
    model::{
        self, paper_question,
        question::{self, PaperWithNum, QuestionSinglePaper, QuestionWithPaper},
        Material, Paper, PaperQuestion, Question, QuestionMaterial, Solution,
    },
    query::question::{PaperQuestionQuery, SectionType},
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
    ) -> anyhow::Result<(Vec<QuestionSinglePaper>, Vec<model::paper::Model>)> {
        if query.paper_ids.is_empty() {
            return Ok((vec![], vec![]));
        }
        let papers = Paper::find_by_ids(&self.db, query.paper_ids.clone()).await?;
        let paper_id_map: HashMap<i32, &model::paper::Model> =
            papers.iter().map(|p| (p.id, p)).collect();
        let pqs = PaperQuestion::find_question_id_by_query(&self.db, query).await?;
        let mut question_id_map: HashMap<i32, model::paper_question::Model> =
            pqs.into_iter().map(|pq| (pq.question_id, pq)).collect();
        let qids = question_id_map.keys().cloned().collect_vec();

        let questions = Question::find_by_ids(&self.db, qids.clone()).await?;
        let mut qm_map = QuestionMaterial::find_by_qids(&self.db, qids.clone()).await?;
        let mids = qm_map.values().flatten().cloned().collect_vec();
        let materials = Material::find_by_ids(&self.db, mids)
            .await
            .context("find materials by ids failed")?;
        let mut id_material_map: HashMap<i32, _> =
            materials.into_iter().map(|m| (m.id, m)).collect();

        let mut solution_map = if query.section_type == SectionType::Together {
            let ss = Solution::find_by_question_ids(&self.db, qids).await?;
            ss.into_iter().into_group_map_by(|s| s.question_id)
        } else {
            HashMap::new()
        };

        let qsp = questions
            .into_iter()
            .map(|q| {
                QuestionSinglePaper::new(
                    q,
                    &paper_id_map,
                    &mut question_id_map,
                    &mut id_material_map,
                    &mut qm_map,
                    &mut solution_map,
                )
            })
            .sorted_by_key(|q| (q.paper.paper.id, q.paper.num))
            .collect_vec();
        Ok((qsp, papers))
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

    pub async fn full_question_by_ids(
        &self,
        ids: Vec<i32>,
    ) -> anyhow::Result<Vec<QuestionWithPaper>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }

        let qs = Question::find_by_ids(&self.db, ids.clone()).await?;

        let all_solutions = Solution::find_by_question_ids(&self.db, ids.clone()).await?;

        let all_pq = PaperQuestion::find_by_question_id_in(&self.db, ids.clone())
            .await
            .context("find question papers failed")?;

        let pid_list: Vec<i32> = all_pq.iter().map(|pq| pq.paper_id).collect();
        let all_papers = Paper::find_by_ids(&self.db, pid_list).await?;

        let solution_map = all_solutions
            .into_iter()
            .into_group_map_by(|s| s.question_id);

        let pq_map = all_pq.into_iter().into_group_map_by(|pq| pq.question_id);

        let paper_map = all_papers
            .into_iter()
            .map(|p| (p.id, p))
            .collect::<HashMap<_, _>>();

        let mut result = Vec::with_capacity(qs.len());

        for q in qs {
            let qid = q.id;

            let ss = solution_map.get(&qid).cloned().unwrap_or_default();

            let papers = pq_map
                .get(&qid)
                .map(|v| {
                    v.iter()
                        .filter_map(|pq| {
                            paper_map
                                .get(&pq.paper_id)
                                .map(|p| PaperWithNum::new(p, pq.sort))
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            result.push(QuestionWithPaper::new(q, papers, Some(ss), None));
        }

        Ok(result)
    }

    pub async fn recommend_question(&self, id: i32) -> anyhow::Result<Vec<QuestionWithPaper>> {
        let q = Question::find_by_id(id).one(&self.db).await?;
        if let Some(q) = q {
            Question::recommend_question(&self.db, &q).await
        } else {
            Ok(vec![])
        }
    }
}
