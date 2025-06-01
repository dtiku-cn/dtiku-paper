use super::GlobalVariables;
use askama::Template;
use dtiku_pay::model::OrderLevel;
use dtiku_pay::model::PayFrom;
use strum::IntoEnumIterator;

#[derive(Template)]
#[template(path = "pay-trade-create.html.jinja")]
pub struct PayTradeCreateTemplate {
    pub global: GlobalVariables,
}
