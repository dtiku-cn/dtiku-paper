use crate::views::{home::HomeTemplate, GlobalVariables};
use anyhow::Context;
use askama::Template;
use spring_web::{
    axum::{
        response::{Html, IntoResponse},
        Extension,
    },
    error::Result,
    get,
};

#[get("/")]
async fn home(Extension(global): Extension<GlobalVariables>) -> Result<impl IntoResponse> {
    let t = HomeTemplate { global };
    Ok(Html(t.render().context("render failed")?))
}
