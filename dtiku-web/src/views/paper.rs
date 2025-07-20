use super::PageExt;
use super::{GlobalVariables, IntoTemplate};
use askama::Template;
use askama_web::WebTemplate;
use dtiku_paper::domain::paper::PaperMode;
use dtiku_paper::domain::question::FullQuestion;
use dtiku_paper::model::question::QuestionExtra;
use dtiku_paper::{
    domain::{
        label::{LabelNode, LabelTree},
        paper::FullPaper,
    },
    model::{self, material, paper, solution, FromType},
    query::paper::ListPaperQuery,
};
use itertools::Itertools;
use spring_sea_orm::pagination::Page;
use std::collections::HashMap;
use strum::IntoEnumIterator;

pub struct PaperType {
    pub id: i16,
    pub name: String,
    pub prefix: String,
    pub pid: i16,
    pub from_ty: FromType,
}

impl PaperType {
    pub fn build_paper_url(&self, query: &ListPaperQuery) -> String {
        format!("/paper?ty={}&lid={}", self.prefix, query.label_id)
    }
}

#[derive(Template, WebTemplate)]
#[template(path = "list-paper.html.min.jinja")]
pub struct ListPaperTemplate {
    pub global: GlobalVariables,
    pub query: ListPaperQuery,
    pub label_tree: LabelTree,
    pub paper_type: PaperType,
    pub label: Option<LabelNode>,
    pub papers: Page<paper::Model>,
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
            papers: list,
        }
    }
}

#[derive(Template, WebTemplate)]
#[template(path = "paper.html.min.jinja")]
pub struct ChapterPaperTemplate {
    pub global: GlobalVariables,
    pub paper: model::paper::Model,
    pub mode: String,
    pub questions: Vec<FullQuestion>,
}

#[derive(Template, WebTemplate)]
#[template(path = "cluster-paper.html.min.jinja")]
pub struct ClusterPaperTemplate {
    pub global: GlobalVariables,
    pub paper: model::paper::Model,
    pub mode: String,
    pub materials: Vec<material::Material>,
    pub questions: Vec<FullQuestion>,
}

impl IntoTemplate<ChapterPaperTemplate> for FullPaper {
    fn to_template(self, global: GlobalVariables) -> ChapterPaperTemplate {
        let mut qid_mid_map = self.qid_mid_map;
        let mut id_m_map: HashMap<i32, material::Material> =
            self.ms.into_iter().map(|m| (m.id, m)).collect();
        let mut qid_ss_map: HashMap<i32, Vec<solution::Model>> = self
            .ss
            .into_iter()
            .map(|m| (m.question_id, m))
            .into_group_map();
        let questions = self
            .qs
            .into_iter()
            .map(|q| {
                FullQuestion::new(
                    qid_mid_map.remove(&q.id).map(|mids| {
                        mids.into_iter()
                            .map(|mid| id_m_map.remove(&mid))
                            .flatten()
                            .collect_vec()
                    }),
                    qid_ss_map.remove(&q.id),
                    self.p.extra.compute_chapter(q.num as i32, true),
                    q,
                )
            })
            .collect_vec();
        ChapterPaperTemplate {
            global,
            mode: self.mode.to_string(),
            paper: self.p,
            questions,
        }
    }
}

impl IntoTemplate<ClusterPaperTemplate> for FullPaper {
    fn to_template(self, global: GlobalVariables) -> ClusterPaperTemplate {
        let mut qid_ss_map: HashMap<i32, Vec<solution::Model>> = self
            .ss
            .into_iter()
            .map(|m| (m.question_id, m))
            .into_group_map();
        let questions = self
            .qs
            .into_iter()
            .map(|q| {
                FullQuestion::new(
                    None,
                    qid_ss_map.remove(&q.id),
                    self.p.extra.compute_chapter(q.num as i32, true),
                    q,
                )
            })
            .collect_vec();
        ClusterPaperTemplate {
            global,
            mode: self.mode.to_string(),
            paper: self.p,
            materials: self.ms,
            questions,
        }
    }
}
