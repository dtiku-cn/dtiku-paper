use crate::{
    domain::keypoint::{KeyPointNode, KeyPointTree},
    model::{key_point, KeyPoint},
};
use anyhow::Context;
use itertools::Itertools;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use spring::plugin::service::Service;
use spring_redis::cache;
use spring_sea_orm::DbConn;
use std::collections::HashMap;

#[derive(Clone, Service)]
pub struct KeyPointService {
    #[inject(component)]
    db: DbConn,
}

impl KeyPointService {
    #[cache("key_point:tree:{paper_type}")]
    pub async fn build_tree_for_paper_type(&self, paper_type: i16) -> anyhow::Result<KeyPointTree> {
        let models = KeyPoint::find_by_paper_type(&self.db, paper_type).await?;

        let pid_children_map = models.iter().into_group_map_by(|m| m.pid);

        Ok(KeyPointTree {
            tree: Self::build_tree(0, &pid_children_map),
        })
    }

    pub async fn find_key_point_by_pid(
        &self,
        paper_type: i16,
        key_point_id: i32,
    ) -> anyhow::Result<Vec<key_point::Model>> {
        KeyPoint::find()
            .filter(
                key_point::Column::Pid
                    .eq(key_point_id)
                    .and(key_point::Column::PaperType.eq(paper_type)),
            )
            .all(&self.db)
            .await
            .with_context(|| format!("find_key_point_by_pid({key_point_id})"))
    }

    fn build_tree(pid: i32, map: &HashMap<i32, Vec<&key_point::Model>>) -> Vec<KeyPointNode> {
        if let Some(children) = map.get(&pid) {
            children
                .iter()
                .map(|m| KeyPointNode {
                    id: m.id,
                    name: m.name.clone(),
                    pid: m.pid,
                    exam_id: m.exam_id,
                    paper_type: m.paper_type,
                    children: Self::build_tree(m.id, map),
                })
                .collect()
        } else {
            vec![]
        }
    }
}
