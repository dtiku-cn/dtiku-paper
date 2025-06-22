use super::GlobalVariables;
use askama::Template;
use dtiku_paper::domain::keypoint::KeyPointTree;

#[derive(Template)]
#[template(path = "shenlun-category.html.min.jinja")]
pub struct ShenlunCategoryTemplate {
    pub global: GlobalVariables,
    pub kp_tree: KeyPointTree,
    pub kp_pid: i32,
    pub kp_id: i32,
}
