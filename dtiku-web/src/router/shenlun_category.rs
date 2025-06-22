use crate::views::{shenlun_category::ShenlunCategoryTemplate, GlobalVariables};
use anyhow::Context;
use askama::Template;
use dtiku_paper::{domain::keypoint::KeyPointTree, service::keypoint::KeyPointService};
use spring_web::{
    axum::{
        response::{Html, IntoResponse},
        Extension,
    },
    error::Result,
    extractor::{Component, Path},
    get,
};

#[get("/shenlun-categories")]
async fn shenlun_category(
    Component(kps): Component<KeyPointService>,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let kp_tree = match global.get_paper_type_by_prefix("shenlun") {
        Some(paper_type) => kps.build_tree_for_paper_type(paper_type.id).await?,
        None => KeyPointTree::none(),
    };
    let kp_pair = kp_tree.default_kp();

    let t = ShenlunCategoryTemplate {
        global,
        kp_tree,
        kp_pid: kp_pair.0,
        kp_id: kp_pair.1,
    };
    Ok(Html(t.render().context("render failed")?))
}

#[get("/shenlun-categories/{kp_pid}/{kp_id}")]
async fn shenlun_category_for_category(
    Path((kp_pid, kp_id)): Path<(i32, i32)>,
    Component(kps): Component<KeyPointService>,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let kp_tree = match global.get_paper_type_by_prefix("shenlun") {
        Some(paper_type) => kps.build_tree_for_paper_type(paper_type.id).await?,
        None => KeyPointTree::none(),
    };
    let t = ShenlunCategoryTemplate {
        global,
        kp_tree,
        kp_pid,
        kp_id,
    };
    Ok(Html(t.render().context("render failed")?))
}
