use crate::views::home::HomeTemplate;
use anyhow::Context;
use askama::Template;
use spring_web::{axum::response::{Html, IntoResponse}, error::Result, get};

#[get("/")]
async fn home() -> Result<impl IntoResponse> {
    println!("index");
    let t = HomeTemplate {};
    Ok(Html(t.render().context("render failed")?))
}
