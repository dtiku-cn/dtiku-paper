#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IdiomReq {
    pub text: Option<String>,
}

impl IdiomReq{
    pub fn to_qs(&self)->String{
        serde_urlencoded::to_string(self).ok().unwrap_or_default()
    }
}