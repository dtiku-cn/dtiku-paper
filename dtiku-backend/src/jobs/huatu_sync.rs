use crate::plugins::fastembed::TxtEmbedding;
use spring::plugin::service::Service;
use spring_sea_orm::DbConn;
use spring_sqlx::ConnectPool;

#[derive(Clone, Service)]
pub struct HuatuSyncService {
    #[inject(component)]
    source_db: ConnectPool,
    #[inject(component)]
    target_db: DbConn,
    #[inject(component)]
    txt_embedding: TxtEmbedding,
}
