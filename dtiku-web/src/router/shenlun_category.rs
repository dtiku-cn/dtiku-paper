use crate::views::{shenlun_category::ShenlunCategoryTemplate, GlobalVariables};
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

#[get("/shenlun-categories")]
async fn shenlun_category(
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let t = ShenlunCategoryTemplate { global };
    Ok(Html(t.render().context("render failed")?))
}
