use std::collections::HashMap;
use std::sync::OnceLock;

use regex::Regex;

static YEAR_REG: OnceLock<Regex> = OnceLock::new();
static PROVINCE_REG: OnceLock<Regex> = OnceLock::new();
static FENBI_MATERIAL_REGEX: OnceLock<Regex> = OnceLock::new();
static SENTENCE_SPLITTER: OnceLock<Regex> = OnceLock::new();

fn get_year_reg() -> &'static Regex {
    YEAR_REG.get_or_init(|| Regex::new(r"(?:^|\D)(19[0-9]{2}|20[0-9]{2})(?:\D|$)").unwrap())
}

fn get_province_reg() -> &'static Regex {
    PROVINCE_REG.get_or_init(|| {
        Regex::new(
            r"(国家|国考|北京|天津|上海|重庆|河北|山西|辽宁|吉林|黑龙江|江苏|浙江|安徽|福建|江西|山东|河南|湖北|湖南|广东|海南|四川|贵州|云南|陕西|甘肃|青海|台湾|内蒙古|广西|西藏|宁夏|新疆|香港|澳门)(省|市|自治区|特别行政区)?"
        ).unwrap()
    })
}

fn get_fenbi_material_regex() -> &'static Regex {
    FENBI_MATERIAL_REGEX.get_or_init(|| Regex::new(r"\[materialid\](\d+)\[\/materialid\]").unwrap())
}

fn get_sentence_splitter() -> &'static Regex {
    SENTENCE_SPLITTER.get_or_init(|| Regex::new(r"(.*?[。！？；：!?;])").unwrap())
}

pub fn pick_year(string: &str) -> Option<i16> {
    if let Some(cap) = get_year_reg().captures(string) {
        let year = &cap[1];
        year.parse().ok()
    } else {
        None
    }
}

pub fn pick_area(string: &str) -> Option<String> {
    if let Some(cap) = get_province_reg().captures(string) {
        let area = &cap[0];
        Some(area.to_string())
    } else {
        None
    }
}

pub fn replace_material_id_ref(
    string: &str,
    mid_num_map: &HashMap<i64, i32>,
) -> (String, Vec<i64>) {
    let mut origin_mids = vec![];
    let content = get_fenbi_material_regex()
        .replace_all(string, |caps: &regex::Captures| {
            let num = &caps[1];
            let origin_material_id = num.parse().unwrap();
            origin_mids.push(origin_material_id);
            let replacement = mid_num_map
                .get(&origin_material_id)
                .map(|v| format!("资料{}", v))
                .unwrap_or_else(|| caps[0].to_string());
            replacement
        })
        .into();
    (content, origin_mids)
}

/// 将一段中文按句子切分
pub fn split_sentences(text: &str) -> Vec<&str> {
    let mut result = Vec::new();

    for cap in get_sentence_splitter().captures_iter(text) {
        let s = cap.get(0).unwrap().as_str().trim();
        if !s.is_empty() {
            result.push(s);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pick_year() {
        assert_eq!(pick_year("2025年真题"), Some(2025));
        assert_eq!(pick_year("2025年真题"), Some(2025));
    }
}