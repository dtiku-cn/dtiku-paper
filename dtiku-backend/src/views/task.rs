use dtiku_base::model::enums::ScheduleTaskType;
use dtiku_base::model::schedule_task::Model;
use sea_orm::prelude::{DateTime, Json};
use serde::Serialize;
use serde_json::Value;
use strum::EnumMessage;

#[derive(Debug, Serialize)]
pub struct ScheduleTask {
    pub id: Option<i32>,
    pub version: i32,
    pub ty: ScheduleTaskType,
    pub desc: &'static str,
    pub active: bool,
    pub context: Json,
    pub run_count: i32,
    pub instances: Json,
    pub created: Option<DateTime>,
    pub modified: Option<DateTime>,
}

impl From<Model> for ScheduleTask {
    fn from(value: Model) -> Self {
        Self {
            id: Some(value.id),
            version: value.version,
            ty: value.ty,
            desc: value.ty.get_message().unwrap_or(""),
            active: value.active,
            context: value.context,
            run_count: value.run_count,
            instances: value.instances,
            created: Some(value.created),
            modified: Some(value.modified),
        }
    }
}

impl From<ScheduleTaskType> for ScheduleTask {
    fn from(value: ScheduleTaskType) -> Self {
        Self {
            id: None,
            version: 0,
            ty: value,
            desc: value.get_message().unwrap_or(""),
            active: false,
            context: Value::Null,
            run_count: 0,
            instances: Value::Null,
            created: None,
            modified: None,
        }
    }
}
