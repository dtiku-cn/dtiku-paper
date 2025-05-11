use chrono::Datelike;
use dtiku_base::service;
use dtiku_paper::domain::exam_category::ExamPaperType;
use paper::PaperType;
use tower_cookies::Cookies;
use user::CurrentUser;

pub mod bbs;
pub mod home;
pub mod paper;
pub mod question;
pub mod user;

pub trait IntoTemplate<T> {
    fn to_template(self, global: GlobalVariables) -> T;
}

#[derive(Debug, Clone)]
pub struct GlobalVariables {
    pub(crate) user: Option<CurrentUser>,
    pub(crate) request_uri: String,
    pub(crate) paper_types: Vec<ExamPaperType>,
    pub(crate) config: service::system_config::Config,
    pub(crate) cookies: Cookies,
}

impl GlobalVariables {
    pub fn now_year(&self) -> i16 {
        let now = chrono::Utc::now();
        now.year() as i16
    }

    pub fn uri_starts_with(&self, prefix: &str) -> bool {
        self.request_uri.starts_with(prefix)
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
}
