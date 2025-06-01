use crate::{
    model::{pay_order, OrderLevel, PayFrom},
    Alipay,
};
use alipay_sdk_rust::biz::{self, BizContenter};
use anyhow::Context;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DbConn};
use spring::plugin::service::Service;

#[derive(Clone, Service)]
pub struct PayOrderService {
    #[inject(component)]
    db: DbConn,
    #[inject(component)]
    alipay: Alipay,
}

impl PayOrderService {
    pub async fn create_order(
        &self,
        user_id: i32,
        level: OrderLevel,
        from: PayFrom,
    ) -> anyhow::Result<()> {
        let order = pay_order::ActiveModel {
            user_id: Set(user_id),
            level: Set(level),
            pay_from: Set(from),
            ..Default::default()
        }
        .insert(&self.db)
        .await
        .context("创建订单失败")?;

        let mut biz_content = biz::TradeCreateBiz::new();
        biz_content.set_subject(format!("公考加油站{}会员", order.level.title()).into());
        biz_content.set_out_trade_no(order.id.to_string().into());
        biz_content.set_total_amount(order.level.amount().into());
        biz_content.set_buyer_id("2088722069264875".into());
        biz_content.set("Timestamp", order.created.timestamp().to_string().into());
        let res = self
            .alipay
            .trade_create(&biz_content)
            .context("订单创建失败")?;
        println!("{}", serde_json::to_string(&res).context("to json failed")?);
        Ok(())
    }
}
