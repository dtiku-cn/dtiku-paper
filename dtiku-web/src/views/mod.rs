use askama::Template;
use axum_extra::extract::CookieJar;
use chinese_number::{ChineseCase, ChineseCountMethod, ChineseVariant, NumberToChinese};
use chrono::{Datelike, NaiveDateTime};
use dtiku_base::{model::user_info, service};
use dtiku_paper::domain::exam_category::ExamPaperType;
use paper::PaperType;
use spring_sea_orm::pagination::Page;
use spring_web::axum::http::{StatusCode, Uri};

pub mod bbs;
pub mod home;
pub mod idiom;
pub mod paper;
pub mod pay;
pub mod question;
pub mod shenlun_category;
pub mod user;

pub trait PageExt {
    fn prev_qs(&self) -> String;
    fn next_qs(&self) -> String;
}

impl<T> PageExt for Page<T> {
    fn prev_qs(&self) -> String {
        format!("page={}&size={}", self.page, self.size)
    }

    fn next_qs(&self) -> String {
        format!("page={}&size={}", self.page + 2, self.size)
    }
}

pub trait IntoTemplate<T> {
    fn to_template(self, global: GlobalVariables) -> T;
}

#[derive(Debug, Clone)]
pub struct GlobalVariables {
    pub(crate) user: Option<user_info::Model>,
    pub(crate) request_uri: Uri,
    pub(crate) original_host: String,
    pub(crate) paper_types: Vec<ExamPaperType>,
    pub(crate) config: service::system_config::Config,
    pub(crate) cookies: CookieJar,
    pub(crate) chars: Vec<char>,
}

#[allow(dead_code)]
impl GlobalVariables {
    pub fn append_params(&self, url: &str, query_str: &str) -> String {
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
        url
    }

    pub fn range(&self, range_end: &u64) -> std::ops::RangeInclusive<u64> {
        (1..=*range_end).into_iter()
    }

    pub fn chinese_num(&self, num: &usize) -> String {
        (*num as i32)
            .to_chinese(
                ChineseVariant::Traditional,
                ChineseCase::Lower,
                ChineseCountMethod::TenThousand,
            )
            .unwrap()
    }

    pub fn now_year(&self) -> i16 {
        let now = chrono::Local::now();
        now.year() as i16
    }

    pub fn date_now(&self) -> String {
        let now = chrono::Local::now();
        now.format("%Y-%m-%d").to_string()
    }

    pub fn format(&self, date: &NaiveDateTime, fmt: &str) -> String {
        date.format(fmt).to_string()
    }

    pub fn format_with_now(&self, date_time: &NaiveDateTime) -> String {
        let end_time = chrono::Local::now().naive_local();

        let start_date = date_time.date();
        let end_date = end_time.date();
        let period = end_date.signed_duration_since(start_date);
        let days = period.num_days();

        // 如果超过一个月（按30天近似）或一年
        if days >= 30 {
            return date_time.format("%Y-%-m-%-d").to_string();
        }

        if days < 1 {
            let duration = end_time - *date_time;
            let seconds = duration.num_seconds();
            let minutes = seconds / 60;

            if minutes > 60 {
                let hours = minutes / 60;
                return format!("{}小时前", hours);
            } else if minutes > 3 {
                return format!("{}分钟前", minutes);
            } else if minutes > 1 {
                return format!("{}分{}前", minutes, seconds % 60);
            } else {
                return format!("{}秒前", seconds);
            }
        }

        if days < 7 {
            return format!("{}天前", days);
        }

        date_time.format("%Y-%-m-%-d").to_string()
    }

    pub fn uri_starts_with(&self, prefix: &str) -> bool {
        self.request_uri.path().starts_with(prefix)
    }

    pub fn current_user_id(&self) -> Option<i32> {
        self.user.as_ref().map(|u| u.id)
    }

    pub fn has_cookie(&self, cookie_name: &str) -> bool {
        self.cookies.get(cookie_name).is_some()
    }

    pub fn cookie(&self, cookie_name: &str) -> String {
        self.cookies
            .get(cookie_name)
            .map(|c| c.value().into())
            .unwrap_or_default()
    }

    pub fn screen_width(&self) -> usize {
        let sw = self.cookie("sw");
        sw.parse().unwrap_or(900)
    }

    pub fn get_paper_type_by_prefix(&self, prefix: &str) -> Option<PaperType> {
        Self::inner_get_paper_type_by_prefix(&self.paper_types, prefix)
    }

    pub fn get_type_by_id(&self, id: i16) -> Option<PaperType> {
        self.paper_types
            .iter()
            .find(|p| p.id == id)
            .map(|p| PaperType {
                id: p.id,
                name: p.name.clone(),
                prefix: p.prefix.clone(),
                pid: p.pid,
                from_ty: p.from_ty.clone(),
            })
    }

    fn inner_get_paper_type_by_prefix(vec: &Vec<ExamPaperType>, prefix: &str) -> Option<PaperType> {
        for p in vec {
            if p.prefix == prefix {
                return Some(PaperType {
                    id: p.id,
                    name: p.name.clone(),
                    prefix: p.prefix.clone(),
                    pid: p.pid,
                    from_ty: p.from_ty.clone(),
                });
            } else if let Some(children) = &p.children {
                let c = Self::inner_get_paper_type_by_prefix(children, prefix);
                if c.is_some() {
                    return c;
                }
            }
        }
        None
    }

    pub(crate) fn new(
        current_user: Option<user_info::Model>,
        request_uri: Uri,
        original_host: String,
        paper_types: Vec<ExamPaperType>,
        config: service::system_config::Config,
        cookies: CookieJar,
    ) -> Self {
        Self {
            user: current_user,
            request_uri,
            original_host,
            paper_types,
            config,
            cookies,
            chars: "ABCDEFGHIJKLMNOPQRSTUVWXYZ".chars().collect(),
        }
    }
}

#[derive(Template)]
#[template(path = "error/err.html.min.jinja")]
pub struct ErrorTemplate<'a> {
    pub status: StatusCode,
    pub msg: &'a str,
    pub original_host: &'a str,
}

#[derive(Template)]
#[template(path = "anti-bot.html.min.jinja")]
pub struct AntiBotTemplate<'a> {
    pub server_secret_key: &'a str,
}
