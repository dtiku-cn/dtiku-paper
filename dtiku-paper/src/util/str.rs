pub fn common_prefix_all(strs: &[&str]) -> String {
    if strs.is_empty() {
        return "".to_string();
    }

    let mut prefix = strs[0].to_string();
    for s in &strs[1..] {
        prefix = common_prefix(&prefix, s).to_string();
        if prefix.is_empty() {
            break;
        }
    }
    prefix
}

pub fn common_prefix<'a>(a: &'a str, b: &'a str) -> &'a str {
    let mut i = 0;
    for (ac, bc) in a.chars().zip(b.chars()) {
        if ac != bc {
            break;
        }
        i += ac.len_utf8();
    }
    &a[..i]
}
