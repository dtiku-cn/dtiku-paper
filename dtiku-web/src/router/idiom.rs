use spring_web::{axum::response::IntoResponse, get};

#[get("/idiom")]
async fn list_idiom() -> impl IntoResponse {
    "idiom-list"
}
