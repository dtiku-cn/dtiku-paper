mod _entities;
pub mod query;
pub mod label;
pub mod material;
pub mod paper;
pub mod exam_category;
pub mod question;
pub mod solution;
pub mod paper_question;
pub mod paper_material;
pub mod key_point;

pub use _entities::prelude::*;
pub use _entities::sea_orm_active_enums::*;
