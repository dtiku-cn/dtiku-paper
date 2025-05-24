use axum_extra::extract::CookieJar;
use chrono::Datelike;
use dtiku_base::{model::user_info, service};
use dtiku_paper::domain::exam_category::ExamPaperType;
use paper::PaperType;
use spring_web::axum::http::Uri;

pub mod bbs;
pub mod home;
pub mod idiom;
pub mod paper;
pub mod question;
pub mod shenlun_category;
pub mod user;

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
