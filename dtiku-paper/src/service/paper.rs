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

    /// 批量查询多个 paper_type 的试卷（避免N+1查询问题）
    pub async fn find_papers_by_types(
        &self,
        exam_id: i16,
        paper_types: &[i16],
    ) -> anyhow::Result<HashMap<i16, Vec<paper::Model>>> {
        if paper_types.is_empty() {
            return Ok(HashMap::new());
        }

        // 批量查询隐藏标签
        let hidden_labels_map =
            Label::find_hidden_label_ids_by_paper_types(&self.db, paper_types).await?;

        // 一次性查询所有 paper_types 的试卷
        let mut filter = paper::Column::ExamId
            .eq(exam_id)
            .and(paper::Column::PaperType.is_in(paper_types.iter().copied()));

        // 收集所有隐藏的 label_id
        let all_hidden_labels: Vec<i32> = hidden_labels_map.values().flatten().copied().collect();
        if !all_hidden_labels.is_empty() {
            filter = filter.and(paper::Column::LabelId.is_not_in(all_hidden_labels));
        }

        let papers = Paper::find()
            .filter(filter)
            .order_by_desc(paper::Column::Year)
            .order_by_desc(paper::Column::Id)
            .all(&self.db)
            .await
            .with_context(|| format!("Paper::find_papers_by_types({:?})", paper_types))?;

        // 按 paper_type 分组，并限制每组最多10个
        Ok(papers
            .into_iter()
            .into_group_map_by(|p| p.paper_type)
            .into_iter()
            .map(|(paper_type, papers)| (paper_type, papers.into_iter().take(10).collect()))
            .collect())
    }
}
