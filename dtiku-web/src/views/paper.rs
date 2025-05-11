use super::{GlobalVariables, IntoTemplate};
use askama::Template;
use dtiku_paper::{
    domain::{
        label::{LabelNode, LabelTree},
        paper::FullPaper,
    },
    model::{
        self,
        paper::{self, PaperExtra},
        FromType,
    },
    query::paper::ListPaperQuery,
};
use spring_sea_orm::pagination::Page;

pub struct PaperType {
    pub id: i16,
    pub name: String,
    pub prefix: String,
    pub pid: i16,
    pub from_ty: FromType,
}

#[derive(Template)]
#[template(path = "list-paper.html")]
pub struct ListPaperTemplate {
    pub global: GlobalVariables,
    pub query: ListPaperQuery,
    pub label_tree: LabelTree,
    pub paper_type: PaperType,
    pub label: Option<LabelNode>,
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
        query: ListPaperQuery,
        label_tree: LabelTree,
        paper_type: PaperType,
        label: Option<LabelNode>,
        list: Page<paper::Model>,
    ) -> Self {
        Self {
            global,
            query,
            label_tree,
            paper_type,
            label,
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
    pub paper: model::paper::Model,
    pub qs: Vec<model::question::Question>,
    pub ms: Vec<model::material::Material>,
    pub ss: Vec<model::solution::Solution>,
}

impl IntoTemplate<PaperTemplate> for FullPaper {
    fn to_template(self, global: GlobalVariables) -> PaperTemplate {
        PaperTemplate {
            global,
            paper: self.p,
            qs: self.qs,
            ms: self.ms,
            ss: self.ss,
        }
    }
}
