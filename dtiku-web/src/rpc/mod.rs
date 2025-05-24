pub mod artalk;

use feignhttp::get;
use serde::{Deserialize, Serialize};

#[get("https://api.weixin.qq.com/sns/userinfo")]
pub async fn wechat_user_info(
    #[query] access_token: &str,
    #[query] openid: &str,
) -> feignhttp::Result<WechatUser> {
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
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
