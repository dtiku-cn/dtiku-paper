use chrono::{Duration, Local, NaiveDateTime};
use dtiku_pay::{
    model::{pay_order, PayFrom, PayOrder},
    service::pay_order::PayOrderService,
};
use spring::tracing;
use spring_job::{extractor::Component as JobComponent, fix_delay};
use spring_sea_orm::DbConn;
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
    StreamComponent(db): StreamComponent<PayOrderService>,
    Json(model): Json<pay_order::Model>,
) {
    let order_id = model.id;
    if let Err(e) = inner_fetch_trade(model, &db).await {
        tracing::error!("fetch_order({order_id}) failed>>>{e:?}")
    }
}

async fn inner_fetch_trade(model: pay_order::Model, svc: &PayOrderService) -> anyhow::Result<()> {
    match model.pay_from {
        PayFrom::Alipay => svc.query_alipay_order(model).await,
        PayFrom::Wechat => svc.query_wechat_order(model).await,
    }
}
