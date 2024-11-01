use askama::Template;

#[derive(Template)]
#[template(path = "list-paper.html")]
pub struct ListPaperTemplate {
    pub papers: Vec<String>
}

#[derive(Template)]
#[template(path = "paper.html")]
pub struct PaperTemplate {
}
