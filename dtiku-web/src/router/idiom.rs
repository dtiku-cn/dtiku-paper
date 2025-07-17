use crate::{
    query::idiom::IdiomReq,
    views::{
        idiom::{IdiomDetailTemplate, ListIdiomTemplate},
        GlobalVariables,
    },
};
use anyhow::Context;
use askama::Template;
use axum_extra::extract::Query;
use dtiku_paper::{domain::label::LabelTree, service::label::LabelService};
use dtiku_stats::{
    model::sea_orm_active_enums::IdiomType,
    query::{IdiomQuery, IdiomSearch},
    service::idiom::IdiomService,
};
use spring_sea_orm::pagination::Pagination;
use spring_web::{
    axum::{
        response::{Html, IntoResponse},
        Extension, Json,
    },
    error::{KnownWebError, Result},
    extractor::{Component, Path},
    get, routes,
};

#[get("/idiom")]
async fn list_idiom(
    Component(ls): Component<LabelService>,
    Component(is): Component<IdiomService>,
    Query(req): Query<IdiomReq>,
    page: Pagination,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    Ok(Html(
        render_list(&ls, &is, IdiomType::Idiom, global, req, page).await?,
    ))
}

#[get("/idiom/like")]
async fn idiom_like(
    Component(is): Component<IdiomService>,
    Query(search): Query<IdiomSearch>,
) -> Result<impl IntoResponse> {
    Ok(Json(is.search_idiom(search).await?))
}

#[get("/word")]
async fn list_word(
    Component(ls): Component<LabelService>,
    Component(is): Component<IdiomService>,
    Query(req): Query<IdiomReq>,
    page: Pagination,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    Ok(Html(
        render_list(&ls, &is, IdiomType::Word, global, req, page).await?,
    ))
}

async fn render_list(
    ls: &LabelService,
    is: &IdiomService,
    ty: IdiomType,
    global: GlobalVariables,
    req: IdiomReq,
    page: Pagination,
) -> anyhow::Result<String> {
    let label_tree = match global.get_paper_type_by_prefix("xingce") {
        Some(paper_type) => ls.find_all_label_by_paper_type(paper_type.id).await?,
        None => LabelTree::none(),
    };
    let origin_req = req.clone();
    let page = if let Some(text) = req.text {
        let search = IdiomSearch { ty, text };
        is.search_idiom_stats(&search, req.labels, &page).await?
    } else {
        let query = IdiomQuery {
            label_id: req.labels,
            page,
        };
        is.get_idiom_stats(ty, &query).await?
    };
    let t = ListIdiomTemplate {
        global,
        model: ty,
        label_tree,
        req: origin_req,
        page,
    };
    t.render().context("render failed")
}

#[routes]
#[get("/word/{text}")]
#[get("/idiom/{text}")]
async fn idiom_detail(
    Component(is): Component<IdiomService>,
    Path(text): Path<String>,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let idiom = is
        .get_idiom_detail(&text)
        .await?
        .ok_or_else(|| KnownWebError::not_found("成语未找到"))?;

    let t = IdiomDetailTemplate { global, idiom };
    Ok(Html(t.render().context("render failed")?))
}
