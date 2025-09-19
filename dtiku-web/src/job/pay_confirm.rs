use crate::service::user::UserService;
use dtiku_pay::model::pay_order;
use spring::tracing;
use spring_stream::{
    extractor::{Component, Json},
    stream_listener,
};

#[stream_listener("pay_order.confirm")]
async fn order_confirm(Component(us): Component<UserService>, Json(model): Json<pay_order::Model>) {
    if model.confirm.is_none() {
        return;
    }
    let user_id = model.user_id;
    let order_level = model.level;
    let r = us.confirm_user(user_id, order_level).await;
    match r {
        Err(e) => {
            tracing::error!("confirm_user({user_id},{order_level}) failed>>>{e:?}");
        }
        Ok(u) => {
            tracing::error!("confirm_user({user_id},{order_level}) success>>>{u:?}");
        }
    }
}
