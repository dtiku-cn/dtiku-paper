use crate::{
    domain::exam_category::ExamPaperType,
    model::{exam_category, ExamCategory},
};
use itertools::Itertools;
use sea_orm::DbConn;
use spring::plugin::service::Service;
use spring_redis::cache;

#[derive(Clone, Service)]
pub struct ExamCategoryService {
    #[inject(component)]
    db: DbConn,
}

impl ExamCategoryService {
    pub async fn find_root_exam(
        &self,
        prefix: &str,
    ) -> anyhow::Result<Option<exam_category::Model>> {
        ExamCategory::find_by_root_prefix(&self.db, prefix).await
    }

    #[cache("paper_types:{root_id}", expire = 86400)]
    pub async fn find_leaf_paper_types(&self, root_id: i16) -> anyhow::Result<Vec<ExamPaperType>> {
        let ecs = ExamCategory::find_all_by_pid(&self.db, root_id).await?;
        let pids: Vec<i16> = ecs.iter().map(|ec| ec.id).collect();
        let leaf = ExamCategory::find_all_by_pids(&self.db, pids).await?;
        let mut grouped = leaf
            .into_iter()
            .sorted_by(|a, b| Ord::cmp(&a.id, &b.id))
            .into_group_map_by(|m| m.pid);
        Ok(ecs
            .into_iter()
            .sorted_by(|a, b| Ord::cmp(&a.id, &b.id))
            .map(|m| ExamPaperType::new(grouped.remove(&m.id), m))
            .collect())
    }
}
