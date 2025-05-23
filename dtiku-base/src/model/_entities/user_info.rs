use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "user_info")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub wechat_id: String,
    pub name: String,
    pub gender: bool,
    pub avatar: String,
    pub created: DateTime,
    pub modified: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
