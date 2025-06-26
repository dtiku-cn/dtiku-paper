pub use super::_entities::assets::*;
use sea_orm::{
    sqlx::types::chrono::Local, ActiveModelBehavior, ActiveValue::Set, ConnectionTrait, DbErr,
};
use spring::async_trait;

#[async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(mut self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if insert {
            self.created = Set(Local::now().naive_local());
        }
        self.modified = Set(Local::now().naive_local());
        Ok(self)
    }
}

impl Model {
    pub fn compute_storage_url(&self) -> String {
        let date = self.created.format("%Y/%m/%d").to_string();
        let src_type = &self.src_type;
        let id = self.id;
        format!("//s.dtiku.cn/{src_type}/{date}/{id}")
    }
}
