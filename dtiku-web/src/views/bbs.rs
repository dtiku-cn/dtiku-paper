use askama::Template;

#[derive(Template)]
#[template(path = "issue/list.html")]
pub struct ListIssueTemplate {

}
