use chrono::Datelike;
use dtiku_paper::domain::exam_category::ExamPaperType;
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
    pub(crate) user: CurrentUser,
    pub(crate) request_uri: String,
    pub(crate) paper_types: Vec<ExamPaperType>,
}

impl GlobalVariables {
    pub fn now_year(&self) -> i16 {
        let now = chrono::Utc::now();
        now.year() as i16
    }
}
