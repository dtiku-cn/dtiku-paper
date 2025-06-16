use crate::views::GetListResult;
use anyhow::Context;
use dtiku_paper::model::KeyPoint;
use spring_sea_orm::DbConn;
use spring_web::{
    axum::{response::IntoResponse, Json},
    error::Result,
    extractor::{Component, Path},
    get,
};

#[get("/api/keypoint/{paper_type}/{pid}")]
async fn list_exam(
    Component(db): Component<DbConn>,
    Path((paper_type, pid)): Path<(i16, i32)>,
) -> Result<impl IntoResponse> {
    let keypoints = KeyPoint::find_by_pid(&db, paper_type, pid)
        .await
        .context("查询KeyPoint失败")?;
    Ok(Json(GetListResult::from(keypoints)))
}
