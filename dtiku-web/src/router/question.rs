use crate::data::question::QuestionSearchTemplate;
use spring_web::get;

#[get("/question/search")]
async fn search_question() -> QuestionSearchTemplate {
    QuestionSearchTemplate {}
}
