use crate::{
    domain::keypoint::{KeyPointNode, KeyPointTree},
    model::{key_point, KeyPoint, QuestionKeyPointStats},
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
        let kp_ids = models.iter().map(|m| m.id).collect_vec();
        let kp_stats = QuestionKeyPointStats::stats_by_key_point_ids(&self.db, kp_ids).await?;

        let pid_children_map = models.iter().into_group_map_by(|m| m.pid);
        let kp_qcount_map: HashMap<i32, i64> = kp_stats
            .into_iter()
            .map(|stats| (stats.key_point_id, stats.total_questions))
            .collect();

        Ok(KeyPointTree {
            tree: Self::build_tree(0, &pid_children_map, &kp_qcount_map),
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

    fn build_tree(
        pid: i32,
        map: &HashMap<i32, Vec<&key_point::Model>>,
        kp_qcount_map: &HashMap<i32, i64>,
    ) -> Vec<KeyPointNode> {
        if let Some(children) = map.get(&pid) {
            children
                .iter()
                .map(|m| KeyPointNode {
                    id: m.id,
                    name: m.name.clone(),
                    pid: m.pid,
                    exam_id: m.exam_id,
                    paper_type: m.paper_type,
                    qcount: kp_qcount_map.get(&m.id).cloned().unwrap_or(0),
                    children: Self::build_tree(m.id, map, kp_qcount_map),
                })
                .collect()
        } else {
            vec![]
        }
    }
}
