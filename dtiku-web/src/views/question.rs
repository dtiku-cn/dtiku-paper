use askama::Template;

use super::GlobalVariables;

#[derive(Template)]
#[template(path = "question-search.html")]
pub struct QuestionSearchTemplate {
    pub global: GlobalVariables,
}

#[derive(Template)]
#[template(path = "question-section.html")]
pub struct QuestionSectionTemplate {
    pub global: GlobalVariables,
}
