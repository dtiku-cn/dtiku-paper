use crate::{
    domain::IdiomStats,
    model::{idiom, idiom_ref_stats, Idiom, IdiomRefStats},
    query::IdiomQuery,
};
use anyhow::Context;
use itertools::Itertools;
use sea_orm::{
    sea_query::IntoCondition, ColumnTrait, DbConn, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect,
};
use spring::plugin::service::Service;
use spring_sea_orm::pagination::{Page, PaginationExt};
use std::collections::HashMap;

#[derive(Clone, Service)]
pub struct IdiomService {
    #[inject(component)]
    db: DbConn,
}

impl IdiomService {
    pub async fn get_idiom_stats(&self, query: &IdiomQuery) -> anyhow::Result<Page<IdiomStats>> {
        let page = IdiomRefStats::find()
            .filter(query.clone().into_condition())
            .order_by_desc(idiom_ref_stats::Column::QuestionCount)
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

        Ok(page.map(|m| IdiomStats::from(id_text_map.get(&m.idiom_id), m)))
    }
}
