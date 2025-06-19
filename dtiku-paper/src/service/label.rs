use crate::{
    domain::label::{LabelNode, LabelTree},
    model::{query::label::LabelQuery, Label},
};
use itertools::Itertools;
use sea_orm::DbConn;
use spring::plugin::service::Service;
use spring_redis::cache;

#[derive(Clone, Service)]
pub struct LabelService {
    #[inject(component)]
    db: DbConn,
}

impl LabelService {
    #[cache("label:tree:{paper_type}", expire = 86400)]
    pub async fn find_all_label_by_paper_type(&self, paper_type: i16) -> anyhow::Result<LabelTree> {
        let root_pid = 0;
        let ls = Label::find_all_by_query(
            &self.db,
            LabelQuery {
                paper_type,
                pid: root_pid,
            },
        )
        .await?;
        let lids = ls.iter().map(|l| l.id).collect_vec();
        let labels = Label::find_by_paper_type_and_pids(&self.db, paper_type, lids).await?;
        let level = !labels.is_empty();
        let mut grouped = labels
            .into_iter()
            .sorted_by(|a, b| Ord::cmp(&a.id, &b.id))
            .into_group_map_by(|m| m.pid);
        let nodes = ls
            .into_iter()
            .sorted_by(|a, b| Ord::cmp(&a.id, &b.id))
            .map(|m| LabelNode::new(grouped.remove(&m.id), m))
            .collect();
        Ok(LabelTree {
            labels: nodes,
            level,
        })
    }
}
