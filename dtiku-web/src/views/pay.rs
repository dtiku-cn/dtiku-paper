use super::GlobalVariables;
use askama::Template;
use dtiku_pay::model::OrderLevel;
use dtiku_pay::model::PayFrom;
use strum::IntoEnumIterator;

#[derive(Template)]
#[template(path = "pay-trade-create.html.jinja")]
pub struct PayTradeCreateTemplate {
    pub global: GlobalVariables,
    pub user_id: i32,
}

#[derive(Template)]
#[template(path = "pay-redirect.html.jinja")]
pub struct PayRedirectTemplate {
    pub global: GlobalVariables,
    pub qrcode_url: String,
    pub pay_from: PayFrom,
}
