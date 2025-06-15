use crate::views::{idiom::ListIdiomTemplate, GlobalVariables};
use anyhow::Context;
use askama::Template;
use dtiku_stats::StatsModelType;
use spring_web::{
    axum::{
        response::{Html, IntoResponse},
        Extension,
    },
    error::Result,
    get,
};

#[get("/idiom")]
async fn list_idiom(Extension(global): Extension<GlobalVariables>) -> Result<impl IntoResponse> {
    let t = ListIdiomTemplate {
        global,
        model: StatsModelType::Idiom,
    };
    Ok(Html(t.render().context("render failed")?))
}

#[get("/word")]
async fn list_word(Extension(global): Extension<GlobalVariables>) -> Result<impl IntoResponse> {
    let t = ListIdiomTemplate {
        global,
        model: StatsModelType::Word,
    };
    Ok(Html(t.render().context("render failed")?))
}
