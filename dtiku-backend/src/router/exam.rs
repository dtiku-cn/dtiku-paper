use crate::views::GetListResult;
use anyhow::Context;
use dtiku_paper::{
    model::{query::label::LabelQuery, ExamCategory, Label},
    service::exam_category::ExamCategoryService,
};
use spring_sea_orm::DbConn;
use spring_web::{
    axum::{response::IntoResponse, Json},
    error::Result,
    extractor::{Component, Path, Query},
    get, post,
};

#[get("/api/exam/{pid}")]
async fn list_exam(
    Component(db): Component<DbConn>,
    Path(pid): Path<i16>,
) -> Result<impl IntoResponse> {
    let exams = ExamCategory::find_children_by_pid(&db, pid)
        .await
        .context("查询ExamCategory失败")?;
    Ok(Json(GetListResult::from(exams)))
}

#[post("/api/exam/{id}/to/{pid}")]
async fn move_exam(
    Component(ecs): Component<ExamCategoryService>,
    Path((id, pid)): Path<(i16, i16)>,
) -> Result<impl IntoResponse> {
    ecs.move_exam(id, pid).await?;
    Ok(Json("success"))
}

#[get("/api/label")]
async fn list_label(
    Component(db): Component<DbConn>,
    Query(query): Query<LabelQuery>,
) -> Result<impl IntoResponse> {
    let exams = Label::find_all_by_query(&db, query)
        .await
        .context("查询Label失败")?;
    Ok(Json(GetListResult::from(exams)))
}
