use anyhow::Context as _;
use fancy_regex::Regex;
use scraper::{Html, Selector};
use std::future::Future;

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

/// 替换 HTML 中所有 <img> 标签的 src 属性
pub async fn async_replace_img_src<F, Fut>(html: &str, replacer: F) -> anyhow::Result<String>
where
    F: Fn(&str) -> Fut,
    Fut: Future<Output = anyhow::Result<String>>,
{
    // 这里用正则，而不是scraper，因为scraper中的ElementRef不是线程安全的
    // 匹配 <img ... src="..."> 或 src='...' 或 src=无引号
    let re = Regex::new(r#"<img\b[^>]*?\bsrc\s*=\s*(['"]?)([^'"\s>]+)\1"#).unwrap();

    let mut result = String::new();
    let mut last_end = 0;

    for caps in re.captures_iter(html) {
        let caps = caps.context("captures failed")?;
        let mat = caps.get(0).unwrap(); // 整个匹配部分 `<img ... src="..."`
        let src = caps.get(2).unwrap().as_str(); // src 的值（不含引号）

        // 替换
        let new_src = replacer(src).await?;

        // 把原 HTML 中 `<img ... src=...` 之前的部分追加
        result.push_str(&html[last_end..mat.start()]);

        // 重写 img 标签片段
        let replaced_img = mat.as_str().replacen(src, &new_src, 1);
        result.push_str(&replaced_img);

        last_end = mat.end();
    }

    // 添加剩余 HTML
    result.push_str(&html[last_end..]);

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

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

    #[tokio::test]
    async fn test_basic_replacement() -> Result<()> {
        let html = r#"<p>Image: <img src="a.png"></p>"#;

        let replaced = async_replace_img_src(html, |src| {
            let src = src.to_string(); // 拷贝一份
            async move { Ok(format!("https://cdn.example.com/{}", src)) }
        })
        .await?;

        assert_eq!(
            replaced,
            r#"<p>Image: <img src="https://cdn.example.com/a.png"></p>"#
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_imgs() -> Result<()> {
        let html = r#"<img src="a.png"><img src='b.jpg'><img src=c.gif>"#;

        let replaced = async_replace_img_src(html, |src| {
            let src = src.to_string(); // 拷贝一份
            async move { Ok(format!("https://cdn.example.com/{}", src)) }
        })
        .await?;

        assert_eq!(
            replaced,
            r#"<img src="https://cdn.example.com/a.png"><img src='https://cdn.example.com/b.jpg'><img src=https://cdn.example.com/c.gif>"#
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_no_img() -> Result<()> {
        let html = r#"<p>No images here</p>"#;

        let replaced =
            async_replace_img_src(html, |_| async { Ok("SHOULD NOT BE CALLED".to_string()) })
                .await?;

        assert_eq!(replaced, html);

        Ok(())
    }

    #[tokio::test]
    async fn test_async_error_propagation() -> Result<()> {
        let html = r#"<img src="fail.png">"#;

        let res = async_replace_img_src(html, |src| {
            let src = src.to_string(); // 拷贝一份
            async move {
                if src == "fail.png" {
                    Err(anyhow::anyhow!("Failed to replace"))
                } else {
                    Ok(src.to_string())
                }
            }
        })
        .await;

        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "Failed to replace");

        Ok(())
    }
}
