use crate::model::paper;
use sea_orm::ColumnTrait;
use sea_orm::{DbConn, EntityTrait, QueryFilter};
use spring::plugin::service::Service;

use crate::model::Paper;

#[derive(Clone, Service)]
pub struct PaperService {
    #[inject(component)]
    db: DbConn,
}

impl PaperService {
    pub async fn search_by_name(&self, name: &str) {
        let _ = Paper::find()
            .filter(paper::Column::Title.contains(name))
            .all(&self.db)
            .await;
        println!("{name}")
    }
}
