use anyhow::Context as _;
use fancy_regex::Regex as FancyRegex;
use once_cell::sync::Lazy;
use regex::Regex;
use scraper::{Html, Selector};
use std::collections::HashSet;
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
    let re =
        FancyRegex::new(r#"<img\b[^>]*?\bsrc\s*=\s*(['"]?)(?!(?:data|blob|cid):)([^'"\s>]+)\1"#)
            .unwrap();

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

static MEDIA_EXT_RE: Lazy<Regex> = Lazy::new(|| {
    // 常见音视频/图片扩展名（忽略大小写），后面允许有查询串或片段
    Regex::new(r"(?i)\.(mp3|wav|ogg|m4a|aac|flac|mp4|webm|mkv|avi|mov|wmv|jpg|jpeg|png|gif|svg|bmp|webp)(?:[?#].*)?$")
        .unwrap()
});
static DATA_URL_RE: Lazy<Regex> = Lazy::new(|| {
    // data:[media_type]/... 的形式
    Regex::new(r"(?i)^data:(audio|video|image)/").unwrap()
});
static URL_IN_STYLE_RE: Lazy<Regex> = Lazy::new(|| {
    // 从 style 中抓 url(...) 的内容
    Regex::new(r#"url\(\s*['"]?([^'")]+)['"]?\s*\)"#).unwrap()
});
static SRCSET_URL_RE: Lazy<Regex> = Lazy::new(|| {
    // 从 srcset 中提取 url（简单拆分，去掉像 "100w"）
    Regex::new(
        r#"(?x)
        (?P<url>[^,\s]+)      # 非空白非逗号片段作为 url
        (?:\s+\d+[wx])?      # 可选的宽度/像素描述符，例如 "100w"
    "#,
    )
    .unwrap()
});

/// 返回从 html 片段中找到的所有疑似媒体资源 URL（去重）
pub fn find_media_sources(fragment: &str) -> Vec<String> {
    let document = Html::parse_fragment(fragment);
    let mut found = HashSet::new();

    // 常用标签 + 属性映射
    let tag_attrs = vec![
        ("img", vec!["src", "srcset"]),
        ("video", vec!["src" /* video 也可能有 <source> */]),
        ("audio", vec!["src"]),
        ("source", vec!["src", "srcset"]),
        ("iframe", vec!["src"]),
        ("embed", vec!["src"]),
        ("object", vec!["data"]),
        ("track", vec!["src"]),
        ("picture", vec![]), // picture 本身无 src，但里面的 img/source 会被捕获
        ("a", vec!["href"]), // 有时直接是媒体链接
    ];

    for (tag, attrs) in tag_attrs {
        if let Ok(sel) = Selector::parse(tag) {
            for el in document.select(&sel) {
                // 检查给定 attrs
                for &attr in &attrs {
                    if let Some(val) = el.value().attr(attr) {
                        // 对 srcset 需要拆分
                        if attr.eq_ignore_ascii_case("srcset") {
                            for cap in SRCSET_URL_RE.captures_iter(val) {
                                if let Some(url) = cap.name("url") {
                                    let s = url.as_str().trim().to_string();
                                    if seems_media(&s) {
                                        found.insert(s);
                                    }
                                }
                            }
                        } else {
                            let s = val.trim().to_string();
                            if seems_media(&s) {
                                found.insert(s);
                            }
                        }
                    }
                }
                // check style attribute for url(...)
                if let Some(style) = el.value().attr("style") {
                    for cap in URL_IN_STYLE_RE.captures_iter(style) {
                        if let Some(url) = cap.get(1) {
                            let s = url.as_str().trim().to_string();
                            if seems_media(&s) {
                                found.insert(s);
                            }
                        }
                    }
                }
            }
        }
    }

    // 额外：全文再扫描一遍，捕捉 data:... media 或直接带扩展名的裸 url（例如在文本或属性里）
    // 用正则从 fragment 中提取类似 http(s)://... 或 /path/xxx.ext 的片段（这里用简单方法）
    let url_like_re = Regex::new(
        r#"(?x)
        (https?://[^\s"'<>]+) |
        (//[^\s"'<>]+) |            # protocol-relative
        (/[^\s"'<>]+?\.[a-zA-Z0-9]{2,4}(?:[?#][^\s"'<>]*)?)  # 以 / 开头并含扩展名的路径
    "#,
    )
    .unwrap();

    for cap in url_like_re.captures_iter(fragment) {
        for i in 1..=3 {
            if let Some(m) = cap.get(i) {
                let s = m.as_str().trim().to_string();
                if seems_media(&s) {
                    found.insert(s);
                }
            }
        }
    }

    // 将 HashSet -> Vec 并返回
    let mut v: Vec<String> = found.into_iter().collect();
    v.sort();
    v
}

/// 简单判断一个 url/字符串是否看起来像媒体资源
fn seems_media(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    if DATA_URL_RE.is_match(s) {
        return true;
    }
    if MEDIA_EXT_RE.is_match(s) {
        return true;
    }
    false
}

/// 便捷函数：是否包含媒体资源
pub fn contains_media(fragment: &str) -> bool {
    !find_media_sources(fragment).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_find_media() {
        let html = r#"
            <div style="background-image: url('/static/bg.jpg');">
                <img src="https://example.org/pic.png" alt="">
                <picture><source srcset="img1.webp 1x, img2.jpg 2x"><img src="fallback.jpg"></picture>
                <video><source src="video.mp4" type="video/mp4"></video>
                <a href="/files/sound.mp3">download</a>
                <div data-custom="data:audio/mp3;base64,AAAA">encoded</div>
            </div>
        "#;
        let found = find_media_sources(html);
        assert!(found.iter().any(|s| s.contains("bg.jpg")));
        assert!(found.iter().any(|s| s.contains("pic.png")));
        assert!(found.iter().any(|s| s.contains("img1.webp")));
        assert!(found.iter().any(|s| s.contains("video.mp4")));
        assert!(found.iter().any(|s| s.contains("sound.mp3")));
        assert!(found.iter().any(|s| s.starts_with("data:audio")));
        assert!(contains_media(html));
    }

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
