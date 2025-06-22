use super::GlobalVariables;
use askama::Template;
use dtiku_paper::domain::keypoint::KeyPointNode;

#[derive(Template)]
#[template(path = "shenlun-category.html.min.jinja")]
pub struct ShenlunCategoryTemplate {
    pub global: GlobalVariables,
    pub kp_tree: Vec<KeyPointNode>,
}
