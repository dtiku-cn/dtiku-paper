use crate::{
    router::EXAM_ID,
    views::{
        home::{HomePapers, HomeTemplate},
        GlobalVariables,
    },
};
use dtiku_paper::{model::paper, service::paper::PaperService};
use dtiku_stats::{
    domain::IdiomStats, model::sea_orm_active_enums::IdiomType, query::IdiomQuery,
    service::idiom::IdiomService,
};
use spring_sea_orm::pagination::{Page, Pagination};
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
        page: Pagination { page: 0, size: 10 },
    };
    let home_papers = vec![
        get_papers(&ps, &global, "xingce").await?,
        get_papers(&ps, &global, "shenlun").await?,
        get_papers(&ps, &global, "gwyms").await?,
    ]
    .into_iter()
    .filter_map(|o| o)
    .collect();
    let idioms = get_idioms(&is, &global, "xingce", IdiomType::Idiom, query).await?;
    let words = get_idioms(&is, &global, "xingce", IdiomType::Word, query).await?;
    Ok(HomeTemplate {
        global,
        home_papers,
        idioms: idioms.content,
        words: words.content,
    })
}

async fn get_papers(
    ps: &PaperService,
    global: &GlobalVariables,
    prefix: &str,
) -> anyhow::Result<Option<HomePapers>> {
    let exam_id = EXAM_ID.get();
    if let Some(paper_type) = global.get_paper_type_by_prefix(prefix) {
        let papers = ps.find_paper_by_type(exam_id, paper_type.id).await?;
        Ok(Some(HomePapers {
            ty: paper_type,
            papers,
        }))
    } else {
        Ok(None)
    }
}

async fn get_idioms(
    is: &IdiomService,
    global: &GlobalVariables,
    prefix: &str,
    ty: IdiomType,
    query: &IdiomQuery,
) -> anyhow::Result<Page<IdiomStats>> {
    let idioms = if let Some(paper_type) = global.get_paper_type_by_prefix(prefix) {
        is.get_idiom_stats(ty, paper_type.id, query).await?
    } else {
        Page::new(vec![], &query.page, 0)
    };
    Ok(idioms)
}
