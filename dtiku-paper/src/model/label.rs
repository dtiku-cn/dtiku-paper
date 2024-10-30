pub use super::_entities::label::*;

use sea_orm::{sqlx::types::chrono::Local, ActiveModelBehavior, ConnectionTrait, DbErr, Set};
use spring::async_trait;

pub struct Label {
    id: i32,
    name: String,
    pid: i32,
}