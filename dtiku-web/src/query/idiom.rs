#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IdiomReq {
    pub text: Option<String>,
    #[serde(default, rename = "lid")]
    pub labels: Vec<i32>,
}

impl IdiomReq {
    pub fn to_qs(&self) -> String {
        serde_html_form::to_string(self).ok().unwrap_or_default()
    }

    pub fn contains_label(&self, label_id: &i32) -> bool {
        self.labels.contains(label_id)
    }

    pub fn to_qs_toggle_label(&self, label_id: &i32) -> String {
        let mut req = self.clone();
        if req.contains_label(label_id) {
            req.labels.retain(|&x| x != *label_id);
        } else {
            req.labels.push(*label_id);
        }
        req.to_qs()
    }
}
