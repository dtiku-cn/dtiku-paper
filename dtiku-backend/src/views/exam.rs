use dtiku_paper::model::FromType;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ExamQuery {
    pub pid: i16,
    pub from_ty: Option<FromType>,
}
