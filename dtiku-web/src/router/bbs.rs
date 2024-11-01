use crate::data::bbs::ListIssueTemplate;
use spring_web::get;

#[get("/bbs")]
async fn list_issue() -> ListIssueTemplate {
    ListIssueTemplate {}
}
