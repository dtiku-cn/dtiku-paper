use super::GlobalVariables;
use askama::Template;
use askama_web::WebTemplate;
use dtiku_pay::model::OrderLevel;
use dtiku_pay::model::PayFrom;
use serde::Serialize;
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
    pub order_id: i32,
    pub qrcode_url: String,
    pub pay_from: PayFrom,
}
