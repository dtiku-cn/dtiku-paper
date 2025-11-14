use crate::{
    query::idiom::IdiomReq,
    router::{error_messages, EXAM_ID},
    views::{
        idiom::{IdiomDetailTemplate, IdiomPrintTemplate, ListIdiomTemplate},
        GlobalVariables,
    },
};
use anyhow::Context;
use axum_extra::extract::Query;
use dtiku_paper::{domain::label::LabelTree, service::label::LabelService};
use dtiku_stats::{
    model::sea_orm_active_enums::IdiomType,
    query::{IdiomQuery, IdiomSearch},
    service::idiom::IdiomService,
};
use spring_sea_orm::pagination::{Page, Pagination};
use spring_web::{
    axum::{
        response::{Html, IntoResponse},
        Extension, Json,
    },
    error::{KnownWebError, Result},
    extractor::{Component, Path},
    get, routes,
};
use std::str::FromStr;

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
    Ok(render_list(&ls, &is, IdiomType::Word, global, req, page).await?)
}

async fn render_list(
    ls: &LabelService,
    is: &IdiomService,
    ty: IdiomType,
    global: GlobalVariables,
    req: IdiomReq,
    pagination: Pagination,
) -> anyhow::Result<ListIdiomTemplate> {
    match global.get_paper_type_by_prefix("xingce") {
        Some(paper_type) => {
            let paper_type: i16 = paper_type.id;
            let label_tree = ls.find_all_label_by_paper_type(paper_type).await?;
            let IdiomReq { text, labels } = req.clone();
            let page = if let Some(text) = text {
                let search = IdiomSearch { ty, text };
                is.search_idiom_stats(&search, paper_type, labels, &pagination)
                    .await?
            } else {
                let query = IdiomQuery {
                    label_id: labels,
                    page: pagination,
                };
                is.get_idiom_stats(ty, paper_type, &query).await?
            };
            Ok(ListIdiomTemplate {
                global,
                model: ty,
                label_tree,
                req,
                page,
            })
        }
        None => Ok(ListIdiomTemplate {
            global,
            model: ty,
            label_tree: LabelTree::none(),
            req,
            page: Page::new(vec![], &pagination, 0),
        }),
    }
}

#[routes]
#[get("/word/{text}")]
#[get("/idiom/{text}")]
async fn idiom_detail(
    Component(is): Component<IdiomService>,
    Path(text): Path<String>,
    Query(req): Query<IdiomReq>,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let exam_id = EXAM_ID.get();
    match global.get_paper_type_by_prefix("xingce") {
        Some(paper_type) => {
            let paper_type: i16 = paper_type.id;
            let idiom = is
                .get_idiom_detail(&text, exam_id, paper_type, req.labels)
                .await?
                .ok_or_else(|| KnownWebError::not_found(error_messages::IDIOM_NOT_FOUND))?;

            Ok(IdiomDetailTemplate { global, idiom })
        }
        None => Err(KnownWebError::bad_request(
            error_messages::INVALID_PAPER_TYPE,
        ))?,
    }
}

#[get("/idiom-print/{model}")]
async fn idiom_print(
    Component(is): Component<IdiomService>,
    Path(model): Path<String>,
    Query(req): Query<IdiomReq>,
    mut pagination: Pagination,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let model = IdiomType::from_str(&model).context("model parse failed")?;
    match global.get_paper_type_by_prefix("xingce") {
        Some(paper_type) => {
            pagination.size = 500;
            let query = IdiomQuery {
                label_id: req.labels,
                page: pagination,
            };
            let page = is.get_idiom_stats(model, paper_type.id, &query).await?;
            Ok(IdiomPrintTemplate {
                global,
                model,
                page,
                url: format!("/idiom-print/{model}"),
            })
        }
        None => Ok(IdiomPrintTemplate {
            global,
            model,
            page: Page::new(vec![], &pagination, 0),
            url: format!("/idiom-print/{model}"),
        }),
    }
}
