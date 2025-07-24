use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod baidu;
pub mod bing;
pub mod sogou;

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchItem {
    pub url: String,
    pub title: String,
    pub desc: String,
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),

    #[error(transparent)]
    ScraperError(#[from] reqwest_scraper::error::ScraperError),
}
