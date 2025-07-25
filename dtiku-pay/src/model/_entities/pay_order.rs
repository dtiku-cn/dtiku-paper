//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.8

use super::sea_orm_active_enums::OrderLevel;
use super::sea_orm_active_enums::PayFrom;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "pay_order")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i32,
    pub level: OrderLevel,
    pub pay_from: PayFrom,
    pub confirm: Option<DateTime>,
    pub created: DateTime,
    pub modified: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
