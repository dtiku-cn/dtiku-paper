use askama::Template;

#[derive(Template)]
#[template(path = "question-search.html")]
pub struct QuestionSearchTemplate {
}