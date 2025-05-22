use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use crate::model::enums::SystemConfigKey;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "user_info")]
pub struct UserInfo {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub gender: bool,
    pub img_url: String,
    pub created: DateTime,
    pub modified: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
