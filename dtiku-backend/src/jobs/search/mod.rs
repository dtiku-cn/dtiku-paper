use serde::{Deserialize, Serialize};

pub mod baidu;
pub mod bing;
pub mod sogou;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchItem {
    pub url: String,
    pub title: String,
    pub desc: String,
}
