use askama::Template;

use super::GlobalVariables;

#[derive(Template)]
#[template(path = "question-search.html")]
pub struct QuestionSearchTemplate {
    pub global: GlobalVariables,
}