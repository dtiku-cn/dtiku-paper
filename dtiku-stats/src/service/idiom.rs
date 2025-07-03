use sea_orm::DbConn;
use spring::plugin::service::Service;

#[derive(Clone, Service)]
pub struct IdiomService {
    #[inject(component)]
    db: DbConn,
}

impl IdiomService {}
