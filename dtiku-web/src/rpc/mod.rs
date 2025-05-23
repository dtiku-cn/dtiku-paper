pub mod artalk;

use feignhttp::get;
use serde::{Deserialize, Serialize};

#[get("https://api.weixin.qq.com/sns/userinfo?access_token={access_token}&openid={openid}")]
pub async fn wechat_user_info(
    #[param] access_token: &str,
    #[param] openid: &str,
) -> feignhttp::Result<WechatUser> {
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WechatUser {
    pub openid: String,
    pub nickname: String,
    pub sex: i64,
    pub province: String,
    pub city: String,
    pub country: String,
    pub headimgurl: String,
    pub privilege: Vec<String>,
    pub unionid: String,
}
