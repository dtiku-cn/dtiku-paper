use super::GlobalVariables;
use askama::Template;
use askama_web::WebTemplate;
use dtiku_base::model::user_info;
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

#[derive(Template, WebTemplate)]
#[template(path = "user/profile.html.jinja")]
pub struct UserProfileTemplate {
    pub global: GlobalVariables,
    pub user: user_info::Model,
}
