use crate::views::paper::{ListPaperTemplate, PaperTemplate};
use anyhow::Context;
use askama::Template;
use spring_web::{axum::response::{Html, IntoResponse}, error::Result, extractor::Path, get};

#[get("/paper")]
async fn list_paper() -> Result<impl IntoResponse> {
    println!("index");
    let t = ListPaperTemplate { papers: vec![] };
    Ok(Html(t.render().context("render failed")?))
}

#[get("/paper/{id}")]
async fn paper_by_id(Path(id): Path<i32>) -> Result<impl IntoResponse> {
    println!("paper: {id}");
    let t = PaperTemplate {};
    Ok(Html(t.render().context("render failed")?))
}
