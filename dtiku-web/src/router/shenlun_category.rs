use crate::views::{shenlun_category::ShenlunCategoryTemplate, GlobalVariables};
use anyhow::Context;
use askama::Template;
use dtiku_paper::service::keypoint::KeyPointService;
use spring_web::{
    axum::{
        response::{Html, IntoResponse},
        Extension,
    },
    error::Result,
    extractor::Component,
    get,
};

#[get("/shenlun-categories")]
async fn shenlun_category(
    Component(kps): Component<KeyPointService>,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let kp_tree = match global.get_paper_type_by_prefix("shenlun") {
        Some(paper_type) => kps.build_tree_for_paper_type(paper_type.id).await?,
        None => vec![],
    };
    let t = ShenlunCategoryTemplate { global, kp_tree };
    Ok(Html(t.render().context("render failed")?))
}
