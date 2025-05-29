use crate::domain::paper::{FullPaper, PaperMode};
use crate::model::Paper;
use crate::model::{paper, Material, Question, QuestionMaterial, Solution};
use crate::query::paper::ListPaperQuery;
use anyhow::Context;
use itertools::Itertools;
use sea_orm::{ColumnTrait, QuerySelect};
use sea_orm::{DbConn, EntityTrait, QueryFilter};
use spring::plugin::service::Service;
use spring_sea_orm::pagination::Page;
use std::collections::HashMap;

#[derive(Clone, Service)]
pub struct PaperService {
    #[inject(component)]
    db: DbConn,
}

impl PaperService {
    pub async fn find_paper_by_id(
        &self,
        id: i32,
        mode: PaperMode,
    ) -> anyhow::Result<Option<FullPaper>> {
        let paper = Paper::find_by_id(id)
            .one(&self.db)
            .await
            .with_context(|| format!("Paper::find_by_id({id}) failed"))?;

        let p = match paper {
            Some(paper) => {
                let qs = Question::find_by_paper_id(&self.db, id).await?;
                let ms = Material::find_by_paper_id(&self.db, id).await?;
                let question_ids = qs.iter().map(|q| q.id).collect_vec();
                let ss = Solution::find_by_question_ids(&self.db, question_ids.clone()).await?;
                let id_map = match paper.extra {
                    paper::PaperExtra::Chapters(_) => {
                        QuestionMaterial::find_by_qids(&self.db, question_ids).await?
                    }
                    _ => HashMap::new(),
                };
                Some(FullPaper::new(mode, paper, qs, ms, ss, id_map))
            }
            None => None,
        };

        Ok(p)
    }

    pub async fn find_paper_by_query(
        &self,
        query: &ListPaperQuery,
    ) -> anyhow::Result<Page<paper::Model>> {
        Paper::find_by_query(&self.db, &query).await
    }

    pub async fn search_by_name(&self, name: &str) -> anyhow::Result<Vec<paper::Model>> {
        Paper::find()
            .filter(paper::Column::Title.contains(name))
            .limit(100)
            .all(&self.db)
            .await
            .with_context(|| format!("Paper::search_by_name failed:{name}"))
    }
}
