use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TextCompare {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Deserialize)]
pub struct WebLabelReq {
    pub url: String,
}
