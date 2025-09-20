use dtiku_pay::model::{OrderLevel, PayFrom};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct TradeCreateQuery {
    #[serde(rename = "level")]
    pub level: OrderLevel,
    #[serde(rename = "from")]
    pub pay_from: PayFrom,
}
