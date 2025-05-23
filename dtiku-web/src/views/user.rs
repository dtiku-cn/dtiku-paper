use askama::Template;

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

#[derive(Template)]
#[template(path = "refresh.html.min.jinja")]
pub struct UserLoginRefreshTemplate {
    pub user: CurrentUser,
}
