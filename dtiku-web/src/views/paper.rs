use super::{GlobalVariables, IntoTemplate};
use askama::Template;
use dtiku_paper::{domain::paper::FullPaper, model::paper};
use spring_sea_orm::pagination::Page;

#[derive(Template)]
#[template(path = "list-paper.html")]
pub struct ListPaperTemplate {
    pub global: GlobalVariables,
    pub papers: Vec<paper::Model>,
    pub size: u64,
    pub page: u64,
    /// the total amount of elements.
    pub total_elements: u64,
    /// the number of total pages.
    pub total_pages: u64,
}

#[derive(Template)]
#[template(path = "paper.html")]
pub struct PaperTemplate {
    pub global: GlobalVariables,
}

impl IntoTemplate<ListPaperTemplate> for Page<paper::Model> {
    fn to_template(self, global: GlobalVariables) -> ListPaperTemplate {
        ListPaperTemplate {
            global,
            papers: self.content,
            size: self.size,
            page: self.page,
            total_elements: self.total_elements,
            total_pages: self.total_pages,
        }
    }
}

impl IntoTemplate<PaperTemplate> for FullPaper {
    fn to_template(self, global: GlobalVariables) -> PaperTemplate {
        todo!()
    }
}
