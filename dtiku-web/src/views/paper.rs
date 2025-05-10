use super::{GlobalVariables, IntoTemplate};
use askama::Template;
use dtiku_paper::{
    domain::{label::LabelTree, paper::FullPaper},
    model::{exam_category, paper},
};
use spring_sea_orm::pagination::Page;

#[derive(Template)]
#[template(path = "list-paper.html")]
pub struct ListPaperTemplate {
    pub global: GlobalVariables,
    pub label_tree: LabelTree,
    pub current_paper_type: exam_category::Model,
    pub papers: Vec<paper::Model>,
    pub size: u64,
    pub page: u64,
    /// the total amount of elements.
    pub total_elements: u64,
    /// the number of total pages.
    pub total_pages: u64,
}
impl ListPaperTemplate {
    pub(crate) fn new(
        global: GlobalVariables,
        label_tree: LabelTree,
        current_paper_type: exam_category::Model,
        list: Page<paper::Model>,
    ) -> Self {
        Self {
            global,
            label_tree,
            current_paper_type,
            papers: list.content,
            size: list.size,
            page: list.page,
            total_elements: list.total_elements,
            total_pages: list.total_pages,
        }
    }
}

#[derive(Template)]
#[template(path = "paper.html")]
pub struct PaperTemplate {
    pub global: GlobalVariables,
}

impl IntoTemplate<PaperTemplate> for FullPaper {
    fn to_template(self, global: GlobalVariables) -> PaperTemplate {
        todo!()
    }
}
