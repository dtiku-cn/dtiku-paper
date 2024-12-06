use sea_orm::DbConn;
use spring::plugin::service::Service;

#[derive(Clone, Service)]
pub struct PaperService {
    #[inject(component)]
    db: DbConn,
}

impl PaperService {
    pub async fn search_by_name(&self, name: &str) {
        println!("{name}")
    }
}
