use askama_axum::IntoResponse;
use spring_web::get;

#[get("/idiom")]
async fn list_idiom() -> impl IntoResponse {
    "idiom-list"
}
