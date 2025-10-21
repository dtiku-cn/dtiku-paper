use crate::{
    domain::exam_category::ExamPaperType,
    model::{exam_category, ExamCategory, FromType},
};
use anyhow::Context;
use itertools::Itertools;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait as _, DbConn, EntityTrait, Statement,
    TransactionTrait as _,
};
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
        let ecs =
            ExamCategory::find_children_by_pid(&self.db, root_id, Some(FromType::Fenbi)).await?;
        let pids: Vec<i16> = ecs.iter().map(|ec| ec.id).collect();
        let leaf = ExamCategory::find_children_by_pids(&self.db, pids).await?;
        let mut grouped = leaf
            .into_iter()
            .sorted_by(|a, b| Ord::cmp(&a.name, &b.name))
            .into_group_map_by(|m| m.pid);
        Ok(ecs
            .into_iter()
            .sorted_by(|a, b| Ord::cmp(&a.name, &b.name))
            .map(|m| ExamPaperType::new(grouped.remove(&m.id), m))
            .collect())
    }

    pub async fn move_exam(&self, id: i16, pid: i16) -> anyhow::Result<()> {
        self.db
            .transaction::<_, (), anyhow::Error>(move |tx| {
                Box::pin(async move {
                    let exam = ExamCategory::find_by_id(id)
                        .one(tx)
                        .await
                        .with_context(|| format!("ExamCategory::find_by_id({id}) failed"))?;

                    if let Some(exam) = exam {
                        exam_category::ActiveModel {
                            id: Set(id),
                            pid: Set(pid),
                            ..Default::default()
                        }
                        .update(tx)
                        .await
                        .with_context(|| format!("ExamCategory::update_pid({id},{pid}) failed"))?;

                        let leaf = ExamCategory::find_leaf_by_pid(tx, id)
                            .await
                            .with_context(|| {
                                format!("ExamCategory::find_leaf_by_pid({id}) failed")
                            })?
                            .into_iter()
                            .map(|m| m.id)
                            .collect_vec();

                        let root = ExamCategory::find_root_by_id(tx, pid)
                            .await
                            .with_context(|| {
                                format!("ExamCategory::find_root_by_id({pid}) failed")
                            })?
                            .unwrap_or(exam);

                        tx.execute(Statement::from_sql_and_values(
                            sea_orm::DatabaseBackend::Postgres,
                            "update label set exam_id = $1 where paper_type = any($2)",
                            [root.id.into(), leaf.into()],
                        ))
                        .await
                        .with_context(|| format!("update label for move_exam({id},{pid})"))?;
                    }

                    Ok(())
                })
            })
            .await?;
        Ok(())
    }
}
