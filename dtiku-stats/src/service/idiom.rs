use crate::{
    domain::{IdiomDetail, IdiomRefStatsWithoutLabel, IdiomStats},
    model::{
        idiom, idiom_ref, idiom_ref_stats, sea_orm_active_enums::IdiomType, Idiom, IdiomRef,
        IdiomRefStats,
    },
    query::{IdiomQuery, IdiomSearch},
};
use anyhow::Context;
use itertools::Itertools;
use sea_orm::{
    prelude::Expr, ColumnTrait, DbConn, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};
use spring::plugin::service::Service;
use spring_sea_orm::pagination::{Page, Pagination, PaginationExt};
use std::collections::HashMap;

#[derive(Clone, Service)]
pub struct IdiomService {
    #[inject(component)]
    db: DbConn,
}

impl IdiomService {
    pub async fn get_idiom_stats(
        &self,
        ty: IdiomType,
        query: &IdiomQuery,
    ) -> anyhow::Result<Page<IdiomStats>> {
        let page = IdiomRefStats::find()
            .select_only()
            .column(idiom_ref_stats::Column::IdiomId)
            .column_as(
                Expr::col(idiom_ref_stats::Column::QuestionCount).sum(),
                "question_count",
            )
            .column_as(
                Expr::col(idiom_ref_stats::Column::PaperCount).sum(),
                "paper_count",
            )
            .filter(query.into_condition(ty))
            .group_by(idiom_ref_stats::Column::IdiomId)
            .order_by_desc(idiom_ref_stats::Column::QuestionCount)
            .into_model::<IdiomRefStatsWithoutLabel>()
            .page(&self.db, &query.page)
            .await
            .context("IdiomRefStats::get_idiom_stats() failed")?;

        if page.is_empty() {
            return Ok(Page::new(vec![], &query.page, page.total_elements));
        }

        let idiom_ids = page.content.iter().map(|m| m.idiom_id).collect_vec();

        let idioms = Idiom::find()
            .select_only()
            .column(idiom::Column::Id)
            .column(idiom::Column::Text)
            .filter(idiom::Column::Id.is_in(idiom_ids))
            .into_tuple::<(i32, String)>()
            .all(&self.db)
            .await
            .context("Idiom::find() failed")?;

        let id_text_map: HashMap<i32, String> = idioms.into_iter().collect();

        Ok(page.map(|m| m.with_idiom(id_text_map.get(&m.idiom_id))))
    }

    pub async fn search_idiom_stats(
        &self,
        search: &IdiomSearch,
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
            .add(idiom_ref_stats::Column::IdiomId.is_in(idiom_ids));
        if !labels.is_empty() {
            filter = filter.and(idiom_ref_stats::Column::LabelId.is_in(labels));
        }
        let stats = IdiomRefStats::find()
            .select_only()
            .column(idiom_ref_stats::Column::IdiomId)
            .column_as(
                Expr::col(idiom_ref_stats::Column::QuestionCount).sum(),
                "question_count",
            )
            .column_as(
                Expr::col(idiom_ref_stats::Column::PaperCount).sum(),
                "paper_count",
            )
            .filter(filter)
            .group_by(idiom_ref_stats::Column::IdiomId)
            .order_by_desc(idiom_ref_stats::Column::QuestionCount)
            .into_model::<IdiomRefStatsWithoutLabel>()
            .all(&self.db)
            .await
            .context("IdiomRefStats::get_idiom_stats() failed")?;

        let id_stats_map: HashMap<i32, IdiomRefStatsWithoutLabel> =
            stats.into_iter().map(|s| (s.idiom_id, s)).collect();

        Ok(page.map(|m| IdiomStats::from(id_stats_map.get(&m.id), m)))
    }

    pub async fn get_idiom_detail(&self, idiom_id: i32) -> anyhow::Result<Option<IdiomDetail>> {
        let idiom = Idiom::find_by_id(idiom_id)
            .one(&self.db)
            .await
            .context("Idiom::get_idiom_detail() failed")?;

        if let Some(idiom) = idiom {
            let refs = IdiomRef::find()
                .filter(idiom_ref::Column::IdiomId.eq(idiom_id))
                .all(&self.db)
                .await
                .context("IdiomRef::find() failed")?;

            Ok(Some(IdiomDetail::new(idiom, refs)))
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
                    .like(search.text)
                    .and(idiom::Column::Ty.eq(search.ty)),
            )
            .limit(10)
            .into_tuple()
            .all(&self.db)
            .await
            .context("Idiom::search_idiom() failed")
    }
}
