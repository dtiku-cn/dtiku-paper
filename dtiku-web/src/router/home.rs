use crate::data::home::HomeTemplate;
use anyhow::Context;
use askama::Template;
use spring_web::{axum::response::IntoResponse, error::Result, get};

#[get("/")]
async fn home() -> Result<impl IntoResponse> {
    println!("index");
    let t = HomeTemplate {};
    Ok(t.render().context("render failed")?)
}
