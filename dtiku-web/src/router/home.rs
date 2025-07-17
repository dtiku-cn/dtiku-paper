use crate::views::{home::HomeTemplate, GlobalVariables};
use dtiku_paper::{model::paper, service::paper::PaperService};
use dtiku_stats::{
    model::sea_orm_active_enums::IdiomType, query::IdiomQuery, service::idiom::IdiomService,
};
use spring_sea_orm::pagination::Pagination;
use spring_web::{
    axum::{response::IntoResponse, Extension},
    error::Result,
    extractor::Component,
    get,
};

#[get("/")]
async fn home(
    Component(ps): Component<PaperService>,
    Component(is): Component<IdiomService>,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let query = &IdiomQuery {
        label_id: vec![],
        page: Pagination { page: 1, size: 10 },
    };
    let xingce = get_papers(&ps, &global, "xingce").await?;
    let shenlun = get_papers(&ps, &global, "shenlun").await?;
    let idioms = is.get_idiom_stats(IdiomType::Idiom, query).await?;
    let words = is.get_idiom_stats(IdiomType::Word, query).await?;
    Ok(HomeTemplate {
        global,
        xingce,
        shenlun,
        idioms: idioms.content,
        words: words.content,
    })
}

async fn get_papers(
    ps: &PaperService,
    global: &GlobalVariables,
    prefix: &str,
) -> anyhow::Result<Vec<paper::Model>> {
    let papers = if let Some(paper_type) = global.get_paper_type_by_prefix(prefix) {
        ps.find_paper_by_type(paper_type.id).await?
    } else {
        vec![]
    };
    Ok(papers)
}
