use crate::{
    domain::keypoint::KeyPointNode,
    model::{key_point, KeyPoint},
};
use itertools::Itertools;
use spring::plugin::service::Service;
use spring_sea_orm::DbConn;
use std::collections::HashMap;

#[derive(Clone, Service)]
pub struct KeyPointService {
    #[inject(component)]
    db: DbConn,
}

impl KeyPointService {
    pub async fn build_tree_for_paper_type(
        &self,
        paper_type: i16,
    ) -> anyhow::Result<Vec<KeyPointNode>> {
        let models = KeyPoint::find_by_paper_type(&self.db, paper_type).await?;

        let pid_children_map = models.iter().into_group_map_by(|m| m.pid);

        Ok(Self::build_tree(0, &pid_children_map))
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
