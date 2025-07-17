use askama::Template;
use askama_web::WebTemplate;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ArtalkUser {
    pub name: String,
    pub email: String,
    pub link: String,
    pub token: String,
    pub is_admin: bool,
}

#[derive(Template, WebTemplate)]
#[template(path = "refresh.html.min.jinja")]
pub struct UserLoginRefreshTemplate {
    pub user: ArtalkUser,
}
