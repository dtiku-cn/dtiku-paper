use spring_stream::{handler::TypedConsumer as _, Consumers};

mod pay_confirm;

pub(crate) fn consumers() -> Consumers {
    Consumers::new().typed_consumer(pay_confirm::order_confirm)
}
