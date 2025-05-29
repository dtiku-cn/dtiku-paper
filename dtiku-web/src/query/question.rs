use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DetailQuery {
    #[serde(default, rename = "onlyComment")]
    pub only_comment: bool,
}
