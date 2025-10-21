use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct TextCompare {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Deserialize)]
pub struct WebExtractReq {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct WebLabelReq {
    pub threshold: Option<f32>,
    pub url: String,
    pub label_text: HashMap<String, String>,
}
