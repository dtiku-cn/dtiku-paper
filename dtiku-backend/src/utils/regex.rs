use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref YEAR_REG: Regex = Regex::new(r"\b(19[0-9]{2}|20[0-9]{2})\b").unwrap();
}

pub fn pick_year(string: &str) -> Option<i16> {
    if let Some(cap) = YEAR_REG.captures(string) {
        let year = &cap[0];
        year.parse().ok()
    } else {
        None
    }
}
