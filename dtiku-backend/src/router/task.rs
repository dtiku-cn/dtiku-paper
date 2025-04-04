use crate::views::task::ScheduleTask as ScheduleTaskView;
use crate::views::GetListResult;
use anyhow::Context;
use dtiku_base::model::enums::ScheduleTaskType;
use dtiku_base::model::{schedule_task, ScheduleTask};
use itertools::Itertools;
use sea_orm::ActiveModelTrait;
use sea_orm::ActiveValue::Set;
use serde_json::json;
use spring_sea_orm::DbConn;
use spring_web::axum::response::IntoResponse;
use spring_web::axum::Json;
use spring_web::error::Result;
use spring_web::extractor::{Component, Path};
use spring_web::{get, post};
use std::collections::HashMap;
use strum::IntoEnumIterator;

#[get("/api/tasks")]
async fn list_tasks(Component(db): Component<DbConn>) -> Result<impl IntoResponse> {
    let tasks = ScheduleTask::find_all(&db).await?;
    let mut ty_map: HashMap<_, _> = tasks.into_iter().map(|c| (c.ty, c)).collect();
    let result: Vec<ScheduleTaskView> = ScheduleTaskType::iter()
        .map(|ty| {
            ty_map
                .remove(&ty)
                .map(|m| m.into())
                .unwrap_or_else(|| ty.into())
        })
        .collect_vec();
    Ok(Json(GetListResult::from(result)))
}

#[get("/api/tasks/:ty")]
async fn get_task(
    Path(ty): Path<ScheduleTaskType>,
    Component(db): Component<DbConn>,
) -> Result<impl IntoResponse> {
    let task = ScheduleTask::find_by_type(&db, ty).await?;
    let model = match task {
        Some(task) => task,
        None => schedule_task::ActiveModel {
            version: Set(1),
            ty: Set(ty),
            run_count: Set(0),
            context: Set(json!(null)),
            instances: Set(json!([])),
            active: Set(false),
            ..Default::default()
        }
        .insert(&db)
        .await
        .context("insert task failed")?,
    };

    Ok(Json(ScheduleTaskView::from(model)))
}

#[post("/api/tasks/:ty")]
async fn start_task(
    Path(ty): Path<ScheduleTaskType>,
    Component(db): Component<DbConn>,
) -> Result<impl IntoResponse> {
    let task = ScheduleTask::find_by_type(&db, ty).await?;
    let model = match task {
        Some(task) => schedule_task::ActiveModel {
            id: Set(task.id),
            context: Set(json!(null)),
            active: Set(true),
            version: Set(task.version + 1),
            ..Default::default()
        }
        .update(&db)
        .await
        .context("update task failed")?,
        None => schedule_task::ActiveModel {
            version: Set(1),
            ty: Set(ty),
            run_count: Set(0),
            context: Set(json!(null)),
            instances: Set(json!([])),
            active: Set(true),
            ..Default::default()
        }
        .insert(&db)
        .await
        .context("insert task failed")?,
    };
    Ok(Json(model))
}
