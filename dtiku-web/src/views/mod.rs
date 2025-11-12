use std::collections::HashMap;

use askama::Template;
use axum_extra::extract::CookieJar;
use chrono::Datelike;
use dtiku_base::{model::user_info, service};
use dtiku_paper::domain::exam_category::ExamPaperType;
use paper::PaperType;
use spring_sea_orm::pagination::Page;
use spring_web::axum::http::{StatusCode, Uri};

pub mod bbs;
pub mod filters;
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
    pub fn range(&self, range_end: &u64) -> std::ops::RangeInclusive<u64> {
        (1..=*range_end).into_iter()
    }

    pub fn now_year(&self) -> i16 {
        let now = chrono::Local::now();
        now.year() as i16
    }

    pub fn date_now(&self) -> String {
        let now = chrono::Local::now();
        now.format("%Y-%m-%d").to_string()
    }

    pub fn uri_starts_with(&self, prefix: &str) -> bool {
        self.request_uri.path().starts_with(prefix)
    }

    pub fn current_user_id(&self) -> Option<i32> {
        self.user.as_ref().map(|u| u.id)
    }

    pub fn user_is_expired(&self) -> bool {
        self.user
            .as_ref()
            .map(|u| u.is_expired())
            .unwrap_or_default()
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

    pub fn match_answer(
        &self,
        user_answer: &Option<HashMap<i32, String>>,
        qid: &i32,
        index0: &usize,
    ) -> bool {
        let user_answer = match user_answer {
            Some(ua) => ua,
            None => return false,
        };
        let answer = match user_answer.get(qid) {
            Some(a) => a,
            None => return false,
        };

        let option_num = index0.to_string();

        answer.contains(&option_num)
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
