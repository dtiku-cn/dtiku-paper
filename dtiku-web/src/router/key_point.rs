use crate::views::GlobalVariables;
use dtiku_paper::service::keypoint::KeyPointService;
use spring_web::{
    axum::{response::IntoResponse, Extension, Json},
    error::Result,
    extractor::{Component, Path},
    get,
};

#[get("/api/kp/{paper_type}/{pid}")]
async fn list_key_point(
    Path((paper_type, pid)): Path<(String, i32)>,
    Component(kps): Component<KeyPointService>,
    Extension(global): Extension<GlobalVariables>,
) -> Result<impl IntoResponse> {
    let key_points = match global.get_paper_type_by_prefix(&paper_type) {
        Some(paper_type) => kps.find_key_point_by_pid(paper_type.id, pid).await?,
        None => vec![],
    };
    Ok(Json(key_points))
}
