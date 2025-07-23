use crate::views::{GlobalVariables, shenlun_category::ShenlunCategoryTemplate};
use dtiku_paper::{
    domain::keypoint::KeyPointTree,
    service::{keypoint::KeyPointService, question::QuestionService},
};
use spring_sea_orm::pagination::Pagination;
use spring_web::{
    axum::{Extension, response::IntoResponse},
    error::Result,
    extractor::{Component, Path},
    get,
};

#[get("/shenlun-categories")]
async fn shenlun_category(
    Component(kps): Component<KeyPointService>,
    Component(qs): Component<QuestionService>,
    Extension(global): Extension<GlobalVariables>,
    page: Pagination,
) -> Result<impl IntoResponse> {
    let kp_tree = match global.get_paper_type_by_prefix("shenlun") {
        Some(paper_type) => kps.build_tree_for_paper_type(paper_type.id).await?,
        None => KeyPointTree::none(),
    };
    let (kp_pid, kp_id) = kp_tree.default_kp();
    inner_shenlun_category(kps, qs, global, kp_tree, kp_pid, kp_id, None, page).await
}

#[get("/shenlun-categories/{kp_pid}/{kp_id}")]
async fn shenlun_category_for_category(
    Path((kp_pid, kp_id)): Path<(i32, i32)>,
    Component(kps): Component<KeyPointService>,
    Component(qs): Component<QuestionService>,
    Extension(global): Extension<GlobalVariables>,
    page: Pagination,
) -> Result<impl IntoResponse> {
    let kp_tree = match global.get_paper_type_by_prefix("shenlun") {
        Some(paper_type) => kps.build_tree_for_paper_type(paper_type.id).await?,
        None => KeyPointTree::none(),
    };
    inner_shenlun_category(kps, qs, global, kp_tree, kp_pid, kp_id, None, page).await
}

#[get("/shenlun-categories/{kp_pid}/{kp_id}/{year}")]
async fn shenlun_category_for_category_and_year(
    Path((kp_pid, kp_id, year)): Path<(i32, i32, i16)>,
    Component(kps): Component<KeyPointService>,
    Component(qs): Component<QuestionService>,
    Extension(global): Extension<GlobalVariables>,
    page: Pagination,
) -> Result<impl IntoResponse> {
    let kp_tree = match global.get_paper_type_by_prefix("shenlun") {
        Some(paper_type) => kps.build_tree_for_paper_type(paper_type.id).await?,
        None => KeyPointTree::none(),
    };
    inner_shenlun_category(kps, qs, global, kp_tree, kp_pid, kp_id, Some(year), page).await
}

async fn inner_shenlun_category(
    kps: KeyPointService,
    qs: QuestionService,
    global: GlobalVariables,
    kp_tree: KeyPointTree,
    kp_pid: i32,
    kp_id: i32,
    year: Option<i16>,
    page: Pagination,
) -> Result<impl IntoResponse> {
    let years = kps.find_year_stats_for_category(kp_id).await?;
    let qids = kps.find_qid_by_kp(kp_id, year, &page).await?;
    let questions = qs.full_question_by_ids(qids).await?;
    Ok(ShenlunCategoryTemplate {
        global,
        kp_tree,
        kp_pid,
        kp_id,
        year,
        years,
        questions,
    })
}
