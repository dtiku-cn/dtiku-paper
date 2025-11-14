use crate::{
    router::EXAM_ID,
    views::{
        home::{HomePapers, HomeTemplate},
        GlobalVariables,
    },
};
use dtiku_paper::service::paper::PaperService;
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
    let exam_id = EXAM_ID.get();

    // 收集所有需要查询的 paper_type_id
    let mut type_ids = Vec::new();
    for paper_type in &global.paper_types {
        if let Some(children) = &paper_type.children {
            for child in children {
                type_ids.push(child.id);
            }
        } else {
            type_ids.push(paper_type.id);
        }
    }

    // 批量查询所有paper（避免N+1问题）
    let all_papers = ps.find_papers_by_types(exam_id, &type_ids).await?;

    // 在内存中组装数据结构
    let mut home_papers = Vec::new();
    for paper_type in &global.paper_types {
        if let Some(children) = &paper_type.children {
            let mut sub_papers = Vec::new();
            for child in children {
                if let Some(papers) = all_papers.get(&child.id) {
                    if !papers.is_empty() {
                        sub_papers.push(HomePapers {
                            ty: child.into(),
                            papers: papers.clone(),
                            sub_papers: vec![],
                        });
                    }
                }
            }
            if !sub_papers.is_empty() {
                home_papers.push(HomePapers {
                    ty: paper_type.into(),
                    papers: vec![],
                    sub_papers,
                });
            }
        } else if let Some(papers) = all_papers.get(&paper_type.id) {
            if !papers.is_empty() {
                home_papers.push(HomePapers {
                    ty: paper_type.into(),
                    papers: papers.clone(),
                    sub_papers: vec![],
                });
            }
        }
    }

    let idioms = get_idioms(&is, &global, "xingce", IdiomType::Idiom, query).await?;
    let words = get_idioms(&is, &global, "xingce", IdiomType::Word, query).await?;
    Ok(HomeTemplate {
        global,
        home_papers,
        idioms: idioms.content,
        words: words.content,
    })
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
