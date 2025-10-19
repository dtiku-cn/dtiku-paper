pub use super::_entities::scraper_solution::*;
use sea_orm::ActiveModelBehavior;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractResult {
    answer: Option<String>,
    analysis: Option<String>,
}

impl ActiveModelBehavior for ActiveModel {}
