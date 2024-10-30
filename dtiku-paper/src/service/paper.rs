use sea_orm::DbConn;
use spring::plugin::service::Service;

#[derive(Clone, Service)]
pub struct PaperService {
    #[component]
    db: DbConn,
}
