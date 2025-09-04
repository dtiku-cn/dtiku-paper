use super::GlobalVariables;
use askama::Template;
use askama_web::WebTemplate;
use dtiku_pay::model::OrderLevel;
use dtiku_pay::model::PayFrom;
use serde::Deserialize;
use strum::IntoEnumIterator;

#[derive(Template, WebTemplate)]
#[template(path = "pay-trade-create.html.jinja")]
pub struct PayTradeCreateTemplate {
    pub global: GlobalVariables,
    pub user_id: i32,
}

#[derive(Template, WebTemplate)]
#[template(path = "pay-redirect.html.jinja")]
pub struct PayRedirectTemplate {
    pub global: GlobalVariables,
    pub qrcode_url: String,
    pub pay_from: PayFrom,
}

/// 支付回调/查询响应
#[derive(Debug, Deserialize)]
pub struct YsmPayNotify {
    /// 商户订单号（商户系统内部订单号）
    pub mch_orderid: String, // string[6,32]

    /// 订单支付金额，单位：分
    pub total_fee: i64, // int

    /// 微信或支付宝内部订单号
    pub transaction_id: String, // string[32]

    /// 易收米平台订单号
    pub ysm_orderid: String, // string[6,32]

    /// 商品描述/标题
    pub description: String, // string[1,127]

    /// 订单状态（仅支付成功才返回：SUCCESS）
    pub state: Option<String>, // string[7]，例如 "SUCCESS"

    /// 支付通道ID
    pub appid: String, // string[1,32]

    /// 支付成功时间戳（仅成功才有）
    pub success_time: Option<i64>, // int (unix 秒)

    /// 当前时间戳
    pub time: i64, // int (unix 秒)

    /// 随机值，避免缓存、防止密钥泄露
    pub nonce_str: String, // string[6,32]

    /// 附加数据（商户自带）
    pub attach: String, // string[0,32]

    /// 签名（必填）
    pub sign: String, // string[64]
}
