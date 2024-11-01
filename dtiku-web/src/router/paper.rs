use crate::data::paper::{ListPaperTemplate, PaperTemplate};
use spring_web::{extractor::Path, get};

#[get("/paper")]
async fn list_paper() -> ListPaperTemplate {
    println!("index");
    ListPaperTemplate { papers: vec![] }
}

#[get("/paper/:id")]
async fn paper_by_id(Path(id): Path<i32>) -> PaperTemplate {
    println!("paper: {id}");
    PaperTemplate {}
}
