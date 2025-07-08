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
    let t = HomeTemplate {
        global,
        xingce: get_papers(&ps, &global, "xingce").await?,
        shenlun: get_papers(&ps, &global, "shenlun").await?,
    };
    Ok(Html(t.render().context("render failed")?))
}

async fn get_papers(
    ps: &PaperService,
    global: &GlobalVariables,
    prefix: &str,
) -> anyhow::Result<Vec<paper::Model>> {
    if let Some(paper_type) = global.get_paper_type_by_prefix(prefix) {
        ps.find_paper_by_type(paper_type.id).await?
    } else {
        vec![]
    }
}
