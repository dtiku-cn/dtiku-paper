use crate::service::user::UserService;
use dtiku_pay::model::{pay_order, OrderStatus};
use spring::tracing;
use spring_stream::{
    extractor::{Component, Json},
    redis::{AutoCommit, RedisConsumerOptions},
    sea_streamer::{ConsumerGroup, ConsumerOptions},
    stream_listener,
};

fn fill_redis_consumer_options(opts: &mut RedisConsumerOptions) {
    let _ = opts
        .set_auto_commit(AutoCommit::Immediate)
        .set_mkstream(true) // 先创建消费组
        .set_consumer_group(ConsumerGroup::new(env!("CARGO_PKG_NAME")));
}

#[stream_listener("pay_order.confirm", mode="LoadBalanced", redis_consumer_options=fill_redis_consumer_options)]
pub(crate) async fn order_confirm(
    Component(us): Component<UserService>,
    Json(model): Json<pay_order::Model>,
) {
    if model.confirm.is_none() {
        tracing::info!("订单未确认:{model:?}");
        return;
    }
    if model.status != OrderStatus::Paid {
        tracing::info!("订单未付款:{model:?}");
        return;
    }
    let user_id = model.user_id;
    let order_level = model.level;
    tracing::info!("user#{user_id}接收到已付款订单{order_level}:{model:?}");
    let r = us.confirm_user(user_id, order_level).await;
    match r {
        Err(e) => {
            tracing::error!("confirm_user({user_id},{order_level}) failed>>>{e:?}");
        }
        Ok(u) => {
            tracing::info!("confirm_user({user_id},{order_level}) success>>>{u:?}");
        }
    }
}
