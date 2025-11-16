use askama::{Result, Values};
use chinese_number::{ChineseCase, ChineseCountMethod, ChineseVariant, NumberToChinese as _};
use chrono::NaiveDateTime;
use pulldown_cmark::{html, Options, Parser};
use scraper::Html;

/// Markdown 转 HTML 的 Askama 过滤器
pub fn markdown(s: &str, _values: &dyn Values) -> Result<String> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(s, options);

    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    Ok(html_output)
}

pub fn hms(seconds: &u64, _values: &dyn Values) -> Result<String> {
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    let s = seconds % 60;

    let result = if h > 0 {
        format!("{}小时{}分{}秒", h, m, s)
    } else if m > 0 {
        format!("{}分{}秒", m, s)
    } else {
        format!("{}秒", s)
    };

    Ok(result)
}

pub fn datetime_fmt(value: &NaiveDateTime, _values: &dyn Values, fmt: &str) -> Result<String> {
    Ok(value.format(fmt).to_string())
}

pub fn format_with_now(value: &NaiveDateTime, _values: &dyn Values) -> Result<String> {
    let end_time = chrono::Local::now().naive_local();

    let start_date = value.date();
    let end_date = end_time.date();
    let period = end_date.signed_duration_since(start_date);
    let days = period.num_days();

    // 如果超过一个月（按30天近似）或一年
    if days >= 30 {
        return Ok(value.format("%Y-%-m-%-d").to_string());
    }

    if days < 1 {
        let duration = end_time - *value;
        let seconds = duration.num_seconds();
        let minutes = seconds / 60;

        if minutes > 60 {
            let hours = minutes / 60;
            return Ok(format!("{}小时前", hours));
        } else if minutes > 3 {
            return Ok(format!("{}分钟前", minutes));
        } else if minutes > 1 {
            return Ok(format!("{}分{}前", minutes, seconds % 60));
        } else {
            return Ok(format!("{}秒前", seconds));
        }
    }

    if days < 7 {
        return Ok(format!("{}天前", days));
    }

    Ok(value.format("%Y-%-m-%-d").to_string())
}

pub fn chinese_num(num: &usize, _values: &dyn Values) -> Result<String> {
    (*num as i128)
        .to_chinese(
            ChineseVariant::Traditional,
            ChineseCase::Lower,
            ChineseCountMethod::TenThousand,
        )
        .map_err(|e| askama::Error::Custom(Box::new(e)))
}

pub fn append_params(url: &str, _values: &dyn Values, query_str: &str) -> Result<String> {
    let mut url = String::from(url);
    if url.contains("?") {
        if !url.ends_with("?") {
            url.push('&');
        }
        url.push_str(query_str)
    } else {
        url.push('?');
        url.push_str(query_str);
    }
    Ok(url)
}

/// 按文本长度截断HTML (公共函数，可供 Rust 代码和模板调用)
/// 保持HTML标签完整性
pub fn truncate_html_by_text_length(html: &str, max_text_chars: usize) -> String {
    let fragment = Html::parse_fragment(html);
    let mut result = String::new();
    let mut text_count = 0;
    let mut tag_stack = Vec::new();
    
    // 递归处理节点
    process_node_for_truncation(
        fragment.root_element(),
        &mut result,
        &mut text_count,
        &mut tag_stack,
        max_text_chars,
    );
    
    // 闭合所有未闭合的标签
    while let Some(tag) = tag_stack.pop() {
        result.push_str("</");
        result.push_str(&tag);
        result.push('>');
    }
    
    result
}

/// 截断 HTML 内容的 Askama 过滤器
/// 按文本字符数截断 HTML，保持标签完整性
pub fn truncate_html(html: &str, _values: &dyn Values, max_text_chars: &usize) -> Result<String> {
    Ok(truncate_html_by_text_length(html, *max_text_chars))
}

/// 处理节点进行截断
fn process_node_for_truncation(
    element: scraper::ElementRef,
    result: &mut String,
    text_count: &mut usize,
    tag_stack: &mut Vec<String>,
    max_length: usize,
) -> bool {
    if *text_count >= max_length {
        return false; // 返回false表示应该停止处理
    }
    
    // 获取标签名
    let tag_name = element.value().name();
    
    // 输出开始标签
    result.push('<');
    result.push_str(tag_name);
    
    // 添加属性
    for (name, value) in element.value().attrs() {
        result.push(' ');
        result.push_str(name);
        result.push_str("=\"");
        result.push_str(&escape_attr(value));
        result.push('"');
    }
    result.push('>');
    
    // 检查是否是自闭合标签
    let is_void = matches!(
        tag_name,
        "area" | "base" | "br" | "col" | "embed" | "hr" | "img" 
        | "input" | "link" | "meta" | "param" | "source" | "track" | "wbr"
    );
    
    if is_void {
        return true; // 继续处理
    }
    
    // 压入标签栈
    tag_stack.push(tag_name.to_string());
    
    // 处理子节点
    for child in element.children() {
        if *text_count >= max_length {
            break;
        }
        
        if let Some(child_element) = scraper::ElementRef::wrap(child) {
            // 是元素节点，递归处理
            if !process_node_for_truncation(child_element, result, text_count, tag_stack, max_length) {
                break;
            }
        } else if let Some(text) = child.value().as_text() {
            // 是文本节点
            let content = text.text.to_string();
            let remaining = max_length.saturating_sub(*text_count);
            
            if remaining > 0 {
                let chars: Vec<char> = content.chars().collect();
                let take = remaining.min(chars.len());
                let truncated: String = chars.iter().take(take).collect();
                
                result.push_str(&escape_html(&truncated));
                *text_count += take;
            }
        }
    }
    
    // 弹出并闭合标签
    if let Some(tag) = tag_stack.pop() {
        result.push_str("</");
        result.push_str(&tag);
        result.push('>');
    }
    
    true // 继续处理
}

/// 简单的HTML转义
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// 简单的属性值转义
fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
