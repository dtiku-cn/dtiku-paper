use crate::views::{home::HomeTemplate, GlobalVariables};
use anyhow::Context;
use askama::Template;
use dtiku_paper::service::paper::PaperService;
use spring_web::{
    axum::{
        response::{Html, IntoResponse},
        Extension,
    },
    error::Result,
    extractor::Component,
    get,
};

#[get("/")]
async fn home(
    Component(ps): Component<PaperService>,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let xingce = if let Some(paper_type) = global.get_paper_type_by_prefix("xingce") {
        ps.find_paper_by_type(paper_type.id).await?
    } else {
        vec![]
    };
    let shenlun = if let Some(paper_type) = global.get_paper_type_by_prefix("shenlun") {
        ps.find_paper_by_type(paper_type.id).await?
    } else {
        vec![]
    };
    let t = HomeTemplate {
        global,
        xingce,
        shenlun,
    };
    Ok(Html(t.render().context("render failed")?))
}
