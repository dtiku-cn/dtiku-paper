use anyhow::Context as _;
use chrono::{Duration, Local, NaiveDateTime};
use dtiku_base::model::user_info;
use dtiku_pay::{
    model::{pay_order, PayFrom},
    service::pay_order::PayOrderService,
};
use sea_orm::DbConn;
use spring::tracing;
use spring_job::{extractor::Component as JobComponent, fix_delay};
use spring_stream::{
    extractor::{Component as StreamComponent, Json},
    stream_listener,
};

#[fix_delay(60)]
async fn fetch_pay_trade_minutely(JobComponent(svc): JobComponent<PayOrderService>) {
    let now = Local::now().naive_local();
    let ten_minutes_ago = now - Duration::minutes(10);
    find_wait_confirm_order(&svc, ten_minutes_ago).await;
}

#[fix_delay(3600)]
async fn fetch_pay_trade_hourly(JobComponent(svc): JobComponent<PayOrderService>) {
    let now = Local::now().naive_local();
    let one_hour_ago = now - Duration::hours(1);
    find_wait_confirm_order(&svc, one_hour_ago).await;
}

async fn find_wait_confirm_order(svc: &PayOrderService, after_time: NaiveDateTime) {
    let r = svc.find_wait_confirm_after(after_time).await;
    match r {
        Ok(orders) => {
            tracing::info!(
                "find {} wait confirm orders after {after_time:?}",
                orders.len()
            );
            for o in orders {
                let order_id = o.id;
                if let Err(e) = inner_fetch_trade(o, svc).await {
                    tracing::error!("fetch_order({order_id}) failed>>>{e:?}")
                }
            }
        }
        Err(e) => {
            tracing::error!("fetch order failed from db>>>{e:?}")
        }
    }
}

#[stream_listener("pay_order")]
async fn trade_fetch(
    StreamComponent(svc): StreamComponent<PayOrderService>,
    StreamComponent(db): StreamComponent<DbConn>,
    Json(model): Json<pay_order::Model>,
) {
    let order_id = model.id;
    let model = match inner_fetch_trade(model, &svc).await {
        Err(e) => {
            tracing::error!("fetch_order({order_id}) failed>>>{e:?}");
            return;
        }
        Ok(model) => model,
    };
    if let Err(e) =
        user_info::ActiveModel::add_expiration_days(&db, model.user_id, model.level.days() as i64)
            .await
    {
        tracing::error!("add expiration days failed for order#{order_id}>>>{e:?}");
    }
}

async fn inner_fetch_trade(
    model: pay_order::Model,
    svc: &PayOrderService,
) -> anyhow::Result<pay_order::Model> {
    let order_id = model.id;
    let pay_from = model.pay_from;
    match pay_from {
        PayFrom::Alipay => svc.query_alipay_order(model).await,
        PayFrom::Wechat => svc.query_wechat_order(model).await,
    }
    .with_context(|| format!("fetch trade failed for order#{order_id} from {pay_from}"))
}
