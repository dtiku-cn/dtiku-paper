mod _entities;
pub mod exam_category;
pub mod key_point;
pub mod label;
pub mod material;
pub mod paper;
pub mod paper_material;
pub mod paper_question;
pub mod query;
pub mod question;
pub mod question_material;
pub mod solution;
pub mod question_keypoint;

pub use _entities::prelude::*;
pub use _entities::sea_orm_active_enums::*;
