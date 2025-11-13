use crate::domain::paper::{FullPaper, PaperMode};
use crate::model::{paper, Material, Question, QuestionMaterial, Solution};
use crate::model::{Label, Paper};
use crate::query::paper::ListPaperQuery;
use anyhow::Context;
use itertools::Itertools;
use sea_orm::{ColumnTrait, QueryOrder, QuerySelect};
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

    pub async fn search_by_name(
        &self,
        paper_type: i16,
        name: &str,
    ) -> anyhow::Result<Vec<paper::Model>> {
        let hidden_labels = Label::find_hidden_label_id_by_paper_type(&self.db, paper_type).await?;
        let mut filter = paper::Column::PaperType
            .eq(paper_type)
            .and(paper::Column::Title.contains(name));
        if !hidden_labels.is_empty() {
            filter = filter.and(paper::Column::LabelId.is_not_in(hidden_labels));
        }
        Paper::find()
            .filter(filter)
            .order_by_desc(paper::Column::Year)
            .limit(100)
            .all(&self.db)
            .await
            .with_context(|| format!("Paper::search_by_name({paper_type},{name}) failed"))
    }

    pub async fn find_paper_by_type(
        &self,
        exam_id: i16,
        paper_type: i16,
    ) -> anyhow::Result<Vec<paper::Model>> {
        let hidden_labels = Label::find_hidden_label_id_by_paper_type(&self.db, paper_type).await?;
        let mut filter = paper::Column::ExamId
            .eq(exam_id)
            .and(paper::Column::PaperType.eq(paper_type));
        if !hidden_labels.is_empty() {
            filter = filter.and(paper::Column::LabelId.is_not_in(hidden_labels));
        }
        Paper::find()
            .filter(filter)
            .order_by_desc(paper::Column::Year)
            .order_by_desc(paper::Column::Id)
            .limit(10)
            .all(&self.db)
            .await
            .with_context(|| format!("Paper::find_paper_by_type({paper_type})"))
    }
}
