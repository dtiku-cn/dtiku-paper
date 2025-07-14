use std::collections::HashMap;

use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref YEAR_REG: Regex = Regex::new(r"\b(19[0-9]{2}|20[0-9]{2})\b").unwrap();
    static ref FENBI_MATERIAL_REGEX: Regex =
        Regex::new(r"资料\[materialid\](\d+)\[\/materialid\]").unwrap();
    static ref SENTENCE_SPLITTER: Regex = Regex::new(r"(.*?[。！？；!?;])").unwrap();
}

pub fn pick_year(string: &str) -> Option<i16> {
    if let Some(cap) = YEAR_REG.captures(string) {
        let year = &cap[0];
        year.parse().ok()
    } else {
        None
    }
}

pub fn replace_material_id_ref(string: &str, mid_num_map: &HashMap<i64, i32>) -> String {
    FENBI_MATERIAL_REGEX
        .replace_all(string, |caps: &regex::Captures| {
            let num = &caps[1];
            let replacement = mid_num_map
                .get(&num.parse().unwrap())
                .map(|v| format!("资料{}", v))
                .unwrap_or_else(|| caps[0].to_string());
            replacement
        })
        .into()
}

/// 将一段中文按句子切分
pub fn split_sentences(text: &str) -> Vec<String> {
    let mut result = Vec::new();

    for cap in SENTENCE_SPLITTER.captures_iter(text) {
        let s = cap.get(0).unwrap().as_str().trim();
        if !s.is_empty() {
            result.push(s.to_string());
        }
    }

    result
}
