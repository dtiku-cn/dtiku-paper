use crate::{
    domain::{IdiomDetail, IdiomRefStatsWithoutLabel, IdiomStats, PaperQuestionRef},
    model::{
        idiom::{self, BriefIdiom},
        idiom_ref, idiom_ref_stats,
        sea_orm_active_enums::IdiomType,
        Idiom, IdiomRef, IdiomRefStats,
    },
    query::{IdiomQuery, IdiomSearch},
};
use anyhow::Context;
use dtiku_paper::model::{Label, Paper, Question};
use itertools::Itertools;
use sea_orm::{
    prelude::Expr, ColumnTrait, DbConn, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};
use spring::plugin::service::Service;
use spring_sea_orm::pagination::{Page, Pagination, PaginationExt};
use std::{cmp::Reverse, collections::HashMap};

#[derive(Clone, Service)]
pub struct IdiomService {
    #[inject(component)]
    db: DbConn,
}

impl IdiomService {
    pub async fn get_idiom_stats(
        &self,
        ty: IdiomType,
        paper_type: i16,
        query: &IdiomQuery,
    ) -> anyhow::Result<Page<IdiomStats>> {
        let mut filter = idiom_ref_stats::Column::Ty.eq(ty);
        if !query.label_id.is_empty() {
            filter = filter.and(idiom_ref_stats::Column::LabelId.is_in(query.label_id.clone()));
        } else {
            let hidden_labels =
                Label::find_hidden_label_id_by_paper_type(&self.db, paper_type).await?;
            if !hidden_labels.is_empty() {
                filter = filter.and(idiom_ref_stats::Column::LabelId.is_not_in(hidden_labels));
            }
        }
        let page = IdiomRefStats::find()
            .select_only()
            .column(idiom_ref_stats::Column::IdiomId)
            .column_as(Expr::cust("SUM(question_count)::BIGINT"), "question_count")
            .column_as(Expr::cust("SUM(paper_count)::BIGINT"), "paper_count")
            .filter(filter)
            .group_by(idiom_ref_stats::Column::IdiomId)
            .order_by_desc(Expr::col("question_count"))
            .into_model::<IdiomRefStatsWithoutLabel>()
            .page(&self.db, &query.page)
            .await
            .context("IdiomRefStats::get_idiom_stats() failed")?;

        if page.is_empty() {
            return Ok(Page::new(vec![], &query.page, page.total_elements));
        }

        let idiom_ids = page.content.iter().map(|m| m.idiom_id).collect_vec();

        let idioms = Idiom::find_brief_in_ids(&self.db, idiom_ids).await?;

        let id_text_map: HashMap<i32, BriefIdiom> = idioms.into_iter().map(|i| (i.id, i)).collect();

        Ok(page.map(|m| m.with_idiom(&id_text_map)))
    }

    pub async fn search_idiom_stats(
        &self,
        search: &IdiomSearch,
        paper_type: i16,
        labels: Vec<i32>,
        pagination: &Pagination,
    ) -> anyhow::Result<Page<IdiomStats>> {
        let page = Idiom::find()
            .filter(
                idiom::Column::Ty
                    .eq(search.ty)
                    .and(idiom::Column::Text.contains(search.text.as_str())),
            )
            .page(&self.db, pagination)
            .await?;

        if page.is_empty() {
            return Ok(Page::new(vec![], &pagination, page.total_elements));
        }

        let idiom_ids = page.content.iter().map(|m| m.id).collect_vec();
        let mut filter = idiom_ref_stats::Column::Ty
            .eq(search.ty)
            .and(idiom_ref_stats::Column::IdiomId.is_in(idiom_ids));
        if !labels.is_empty() {
            filter = filter.and(idiom_ref_stats::Column::LabelId.is_in(labels));
        } else {
            let hidden_labels =
                Label::find_hidden_label_id_by_paper_type(&self.db, paper_type).await?;
            if !hidden_labels.is_empty() {
                filter = filter.and(idiom_ref_stats::Column::LabelId.is_not_in(hidden_labels));
            }
        }
        let stats = IdiomRefStats::find()
            .select_only()
            .column(idiom_ref_stats::Column::IdiomId)
            .column_as(Expr::cust("SUM(question_count)::BIGINT"), "question_count")
            .column_as(Expr::cust("SUM(paper_count)::BIGINT"), "paper_count")
            .filter(filter)
            .group_by(idiom_ref_stats::Column::IdiomId)
            .order_by_desc(Expr::col("question_count"))
            .into_model::<IdiomRefStatsWithoutLabel>()
            .all(&self.db)
            .await
            .context("IdiomRefStats::search_idiom_stats() failed")?;

        let id_stats_map: HashMap<i32, IdiomRefStatsWithoutLabel> =
            stats.into_iter().map(|s| (s.idiom_id, s)).collect();

        Ok(page.map(|m| IdiomStats::from(id_stats_map.get(&m.id), m)))
    }

    pub async fn get_idiom_detail(
        &self,
        text: &str,
        labels: Vec<i32>,
    ) -> anyhow::Result<Option<IdiomDetail>> {
        let idiom = Idiom::find_by_text(&self.db, text)
            .await
            .with_context(|| format!("Idiom::get_idiom_detail({text}) failed"))?;

        if let Some(idiom) = idiom {
            let jyc = Idiom::find_by_texts(&self.db, idiom.content.jyc().clone(), &labels).await?;
            let fyc = Idiom::find_by_texts(&self.db, idiom.content.fyc().clone(), &labels).await?;

            let mut ref_filter = idiom_ref::Column::IdiomId.eq(idiom.id);
            if !labels.is_empty() {
                ref_filter = ref_filter.and(idiom_ref::Column::LabelId.is_in(labels));
            }
            let refs = IdiomRef::find()
                .filter(ref_filter)
                .all(&self.db)
                .await
                .context("IdiomRef::find() failed")?;

            let paper_ids = refs.iter().map(|r| r.paper_id).collect_vec();
            let question_ids = refs.iter().map(|r| r.question_id).collect_vec();

            let papers = Paper::find_by_ids(&self.db, paper_ids).await?;
            let questions = Question::find_by_ids_with_solutions(&self.db, question_ids).await?;

            let id_paper: HashMap<i32, _> = papers.into_iter().map(|p| (p.id, p)).collect();
            let id_question: HashMap<i32, _> = questions.into_iter().map(|q| (q.id, q)).collect();

            let refs = refs
                .into_iter()
                .map(|r| {
                    let p = id_paper.get(&r.paper_id);
                    let q = id_question.get(&r.question_id);
                    PaperQuestionRef::new(r, p, q)
                })
                .sorted_by_key(|r| (Reverse(r.paper.year), r.paper.id))
                .collect();

            Ok(Some(IdiomDetail::new(idiom, refs, jyc, fyc)))
        } else {
            Ok(None)
        }
    }

    pub async fn search_idiom(&self, search: IdiomSearch) -> anyhow::Result<Vec<String>> {
        Idiom::find()
            .select_only()
            .column(idiom::Column::Text)
            .filter(
                idiom::Column::Text
                    .starts_with(search.text)
                    .and(idiom::Column::Ty.eq(search.ty)),
            )
            .limit(10)
            .into_tuple()
            .all(&self.db)
            .await
            .context("Idiom::search_idiom() failed")
    }
}
