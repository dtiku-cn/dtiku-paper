use scraper::{Html, Selector};

/// 替换 HTML 中所有 <img> 标签的 src 属性
pub fn replace_img_src<F>(html: &str, replacer: F) -> String
where
    F: Fn(&str) -> String,
{
    let document = Html::parse_fragment(html);
    let selector = Selector::parse("img").unwrap();

    let mut output = html.to_string();

    for img in document.select(&selector) {
        if let Some(src_attr) = img.value().attr("src") {
            let new_src = replacer(src_attr);

            // 为了支持各种 src 格式，保守替换所有可能的 src="..." 形式
            let variants = vec![
                format!("src=\"{}\"", src_attr),
                format!("src='{}'", src_attr),
                format!("src={}", src_attr), // 少引号的容错
            ];

            for old in variants {
                if output.contains(&old) {
                    let new = format!("src=\"{}\"", new_src);
                    output = output.replace(&old, &new);
                    break;
                }
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_img_src_basic() {
        let input = r#"<p>Hello</p><img src="a.jpg" alt="pic"><img src='b.png'>"#;
        let expected = r#"<p>Hello</p><img src="https://cdn.com/a.jpg" alt="pic"><img src="https://cdn.com/b.png">"#;

        let result = replace_img_src(input, |src| format!("https://cdn.com/{}", src));

        assert_eq!(result, expected);
    }

    #[test]
    fn test_replace_img_src_mixed_attributes() {
        let input = r#"<img width="100" src=a.jpg><img src="b.png" height="200">"#;
        let expected = r#"<img width="100" src="https://cdn.com/a.jpg"><img src="https://cdn.com/b.png" height="200">"#;

        let result = replace_img_src(input, |src| format!("https://cdn.com/{}", src));

        assert_eq!(result, expected);
    }

    #[test]
    fn test_replace_img_src_no_img() {
        let input = r#"<p>No image here</p>"#;
        let result = replace_img_src(input, |src| format!("https://cdn.com/{}", src));
        assert_eq!(result, input);
    }
}
