use feignhttp::{get, post};
use serde::{Deserialize, Serialize};

/// 获取公众号 access_token
/// https://developers.weixin.qq.com/doc/service/api/webdev/access/api_snsaccesstoken.html
#[get("https://api.weixin.qq.com/cgi-bin/token")]
pub async fn get_access_token(
    #[query] grant_type: &str,
    #[query] appid: &str,
    #[query] secret: &str,
) -> feignhttp::Result<AccessTokenResponse> {
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenResponse {
    pub access_token: Option<String>,
    pub expires_in: Option<i64>,
    pub errcode: Option<i32>,
    pub errmsg: Option<String>,
}

/// 创建带参数的二维码
/// https://developers.weixin.qq.com/doc/service/api/qrcode/qrcodes/api_createqrcode.html
#[post("https://api.weixin.qq.com/cgi-bin/qrcode/create")]
pub async fn create_qrcode(
    #[query] access_token: &str,
    #[body] body: CreateQrcodeRequest,
) -> feignhttp::Result<CreateQrcodeResponse> {
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateQrcodeRequest {
    pub expire_seconds: i32,
    pub action_name: String,
    pub action_info: ActionInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionInfo {
    pub scene: Scene,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene_str: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateQrcodeResponse {
    pub ticket: Option<String>,
    pub expire_seconds: Option<i32>,
    pub url: Option<String>,
    pub errcode: Option<i32>,
    pub errmsg: Option<String>,
}

/// 获取用户基本信息（公众号）
/// https://developers.weixin.qq.com/doc/service/api/usermanage/userinfo/api_userinfo.html
#[get("https://api.weixin.qq.com/cgi-bin/user/info")]
pub async fn get_user_info(
    #[query] access_token: &str,
    #[query] openid: &str,
    #[query] lang: &str,
) -> feignhttp::Result<WechatMpUser> {
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WechatMpUser {
    pub openid: String,
    pub nickname: String,
    pub sex: i32,
    pub province: String,
    pub city: String,
    pub country: String,
    pub headimgurl: String,
    pub subscribe: Option<i32>,
    pub subscribe_time: Option<i64>,
    pub unionid: Option<String>,
    pub errcode: Option<i32>,
    pub errmsg: Option<String>,
}

/// 获取网页授权用户信息
/// https://developers.weixin.qq.com/doc/service/api/webdev/access/api_snsuserinfo.html
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

impl CreateQrcodeRequest {
    /// 创建临时二维码请求（字符串场景值，用于登录）
    pub fn new_temp_str(scene_str: String, expire_seconds: i32) -> Self {
        Self {
            expire_seconds,
            action_name: "QR_STR_SCENE".to_string(),
            action_info: ActionInfo {
                scene: Scene {
                    scene_id: None,
                    scene_str: Some(scene_str),
                },
            },
        }
    }

    /// 创建临时二维码请求（整数场景值）
    pub fn new_temp_id(scene_id: u32, expire_seconds: i32) -> Self {
        Self {
            expire_seconds,
            action_name: "QR_SCENE".to_string(),
            action_info: ActionInfo {
                scene: Scene {
                    scene_id: Some(scene_id),
                    scene_str: None,
                },
            },
        }
    }

    /// 创建永久二维码请求（整数场景值）
    /// 注意：永久二维码数量有限制（最多10万个）
    pub fn new_permanent_id(scene_id: u32) -> Self {
        Self {
            expire_seconds: 0, // 永久二维码此字段无意义
            action_name: "QR_LIMIT_SCENE".to_string(),
            action_info: ActionInfo {
                scene: Scene {
                    scene_id: Some(scene_id),
                    scene_str: None,
                },
            },
        }
    }

    /// 创建永久二维码请求（字符串场景值）
    /// 注意：永久二维码数量有限制（最多10万个）
    pub fn new_permanent_str(scene_str: String) -> Self {
        Self {
            expire_seconds: 0, // 永久二维码此字段无意义
            action_name: "QR_LIMIT_STR_SCENE".to_string(),
            action_info: ActionInfo {
                scene: Scene {
                    scene_id: None,
                    scene_str: Some(scene_str),
                },
            },
        }
    }
}
