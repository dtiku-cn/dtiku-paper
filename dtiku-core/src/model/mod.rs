pub mod label;
pub mod paper;
pub mod question;
pub mod material;
pub mod solution;

pub struct PaperQuestion {
    paper_id: i32,
    question_id: i32,
    sort: i16,
}

pub struct PaperMaterial {
    paper_id: i32,
    material_id: i32,
    sort: i16,
}

pub struct QuestionMaterial {
    question_id: i32,
    material_id: i32,
}