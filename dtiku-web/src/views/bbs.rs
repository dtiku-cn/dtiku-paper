use super::filters;
use super::GlobalVariables;
use super::PageExt;
use crate::plugins::grpc_client::artalk::VoteStats;
use askama::Template;
use askama_web::WebTemplate;
use chrono::NaiveDateTime;
use dtiku_base::model::user_info;
use dtiku_bbs::model::issue::CollectIssueMarkdown;
use dtiku_bbs::model::issue::ListIssue;
use dtiku_bbs::model::{issue, IssueQuery, TopicType};
use spring_sea_orm::pagination::Page;
use strum::IntoEnumIterator;
use scraper::Html;

#[derive(Template, WebTemplate)]
#[template(path = "issue/list.html.min.jinja")]
pub struct ListIssueTemplate {
    pub global: GlobalVariables,
    pub page: Page<FullIssue>,
    pub query: IssueQuery,
    pub pin_issues: Vec<ListIssue>,
}

pub struct FullIssue {
    pub id: i32,
    pub topic: TopicType,
    pub title: String,
    pub toc: String,
    pub markdown: String,
    pub html: String,
    pub user_id: i32,
    pub pin: bool,
    pub collect: bool,
    pub paid: bool,
    pub created: NaiveDateTime,
    pub modified: NaiveDateTime,
    pub view: i32,
    pub comment: i32,
    pub vote_up: i64,
    pub vote_down: i64,
    pub user: Option<user_info::Model>,
}

impl FullIssue {
    pub fn new(
        issue: issue::Model,
        page_pv: &std::collections::HashMap<String, i32>,
        page_comment: &std::collections::HashMap<String, i32>,
        votes: &std::collections::HashMap<String, VoteStats>,
        id_user_map: &mut std::collections::HashMap<i32, user_info::Model>,
    ) -> Self {
        let key = format!("/bbs/issue/{}", issue.id);
        let (toc, markdown) = if issue.collect {
            let collect = serde_json::from_str::<CollectIssueMarkdown>(&issue.markdown)
                .expect("collect issue require CollectIssueMarkdown");
            (collect.toc, collect.content)
        } else {
            ("".to_string(), issue.markdown)
        };
        FullIssue {
            user: id_user_map.get(&issue.user_id).cloned(),
            id: issue.id,
            title: issue.title,
            topic: issue.topic,
            toc,
            markdown: markdown,
            html: issue.html,
            user_id: issue.user_id,
            pin: issue.pin,
            collect: issue.collect,
            paid: issue.paid,
            created: issue.created,
            modified: issue.modified,
            view: page_pv.get(&key).unwrap_or(&0).to_owned(),
            comment: page_comment.get(&key).unwrap_or(&0).to_owned(),
            vote_up: votes.get(&key).map(|v| v.vote_up).unwrap_or_default(),
            vote_down: votes.get(&key).map(|v| v.vote_down).unwrap_or_default(),
        }
    }

    pub fn new_for_list(
        issue: issue::ListIssue,
        page_pv: &std::collections::HashMap<String, i32>,
        page_comment: &std::collections::HashMap<String, i32>,
        votes: &std::collections::HashMap<String, VoteStats>,
        id_user_map: &mut std::collections::HashMap<i32, user_info::Model>,
    ) -> Self {
        let key = format!("/bbs/issue/{}", issue.id);
        FullIssue {
            user: id_user_map.get(&issue.user_id).cloned(),
            id: issue.id,
            title: issue.title,
            topic: issue.topic,
            toc: "".to_string(),
            markdown: "".to_string(),
            html: "".to_string(),
            user_id: issue.user_id,
            pin: issue.pin,
            collect: issue.collect,
            paid: issue.paid,
            created: issue.created,
            modified: issue.modified,
            view: page_pv.get(&key).unwrap_or(&0).to_owned(),
            comment: page_comment.get(&key).unwrap_or(&0).to_owned(),
            vote_up: votes.get(&key).map(|v| v.vote_up).unwrap_or_default(),
            vote_down: votes.get(&key).map(|v| v.vote_down).unwrap_or_default(),
        }
    }

    pub fn author_name(&self) -> String {
        self.user
            .as_ref()
            .map_or_else(|| "未知用户".to_string(), |u| u.name.clone())
    }

    /// 截断HTML内容用于付费墙预览
    /// 使用scraper库解析HTML并截取指定数量的文本字符
    pub fn truncate_html(&mut self) {
        const PREVIEW_TEXT_LENGTH: usize = 300; // 文本字符数
        
        // 解析HTML获取纯文本
        let fragment = Html::parse_fragment(&self.html);
        let full_text: String = fragment.root_element().text().collect();
        
        // 如果文本长度在限制内，不截断
        if full_text.chars().count() <= PREVIEW_TEXT_LENGTH {
            return;
        }
        
        // 按字符数截断HTML
        let truncated_html = truncate_html_by_text_length(&self.html, PREVIEW_TEXT_LENGTH);
        self.html = truncated_html;
    }
}

/// 按文本长度截断HTML
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

#[derive(Template, WebTemplate)]
#[template(path = "issue/issue.html.min.jinja")]
pub struct IssueTemplate {
    pub global: GlobalVariables,
    pub issue: FullIssue,
}

#[derive(Template, WebTemplate)]
#[template(path = "issue/paywall-content.html.jinja")]
pub struct PaywallContentTemplate {
    pub content: String,
    pub is_logged_in: bool,
}

#[derive(Template, WebTemplate)]
#[template(path = "issue/issue-editor.html.min.jinja")]
pub struct IssueEditorTemplate {
    pub global: GlobalVariables,
    pub issue: Option<FullIssue>,
}

trait TopicSelected {
    fn is_topic(&self, topic: &TopicType) -> bool;
}

impl TopicSelected for Option<FullIssue> {
    fn is_topic(&self, topic: &TopicType) -> bool {
        let t = self.as_ref().map(|i| i.topic);
        t == Some(topic.clone())
    }
}
