#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub name: String,
    pub avatar: String,
}

impl CurrentUser {
    pub fn is_expired(&self) -> bool {
        true
    }
}
