//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.8

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use crate::model::enums::ScheduleTaskType;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "schedule_task")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub version: i32,
    pub ty: ScheduleTaskType,
    pub active: bool,
    #[sea_orm(column_type = "JsonBinary")]
    pub context: Json,
    pub run_count: i32,
    #[sea_orm(column_type = "JsonBinary")]
    pub instances: Json,
    pub created: DateTime,
    pub modified: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
