use regex::{Regex, RegexBuilder};
use serde::Deserialize;
use strum::{AsRefStr, Display, EnumIter, EnumMessage, EnumProperty, EnumString};

#[derive(
    Debug,
    Clone,
    Copy,
    Deserialize,
    Display,
    EnumIter,
    AsRefStr,
    EnumString,
    EnumMessage,
    EnumProperty,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum StatsModelType {
    #[strum(props(
        text = "成语",
        regex = "(?<![\\u4e00-\\u9fa5])([\\u4e00-\\u9fa5]{4})(?![\\u4e00-\\u9fa5])"
    ))]
    Idiom,

    #[strum(props(
        text = "词语",
        regex = "(?<![\\u4e00-\\u9fa5])([\\u4e00-\\u9fa5]{2})(?![\\u4e00-\\u9fa5])"
    ))]
    Word,
}

impl StatsModelType {
    pub fn text(&self) -> &'static str {
        self.get_str("text").unwrap_or_default()
    }

    pub fn title(&self) -> Regex {
        let regex = self.get_str("regex").unwrap_or_default();

        RegexBuilder::new(regex).multi_line(true).build().unwrap()
    }
}
