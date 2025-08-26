use askama::Values;
use pulldown_cmark::{html, Options, Parser};

pub type FilterResult<T> = Result<T, askama::Error>;

/// Markdown 转 HTML 的 Askama 过滤器
pub fn markdown(s: &str, _args: &dyn Values) -> FilterResult<String> {
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
