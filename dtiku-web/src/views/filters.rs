use askama::{Result, Values};
use chinese_number::{ChineseCase, ChineseCountMethod, ChineseVariant, NumberToChinese as _};
use chrono::NaiveDateTime;
use pulldown_cmark::{html, Options, Parser};

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
