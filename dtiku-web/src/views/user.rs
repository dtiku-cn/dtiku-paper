use askama::Template;
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub name: String,
    pub avatar: String,
}

impl CurrentUser {
    pub fn is_expired(&self) -> bool {
        true
    }

    pub fn due_time(&self) -> String {
        "2023-10-01".to_string()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ArtalkUser {
    pub name: String,
    pub email: String,
    pub link: String,
    pub token: String,
    pub is_admin: bool,
}

#[derive(Template)]
#[template(path = "refresh.html.min.jinja")]
pub struct UserLoginRefreshTemplate {
    pub user: ArtalkUser,
}
