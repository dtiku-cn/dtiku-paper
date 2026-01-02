use crate::{
    domain::keypoint::{KeyPointNode, KeyPointPath, KeyPointTree},
    model::{
        key_point, question_keypoint, question_keypoint_stats, KeyPoint, QuestionKeyPoint,
        QuestionKeyPointStats,
    },
};
use anyhow::Context;
use itertools::Itertools;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use spring::plugin::service::Service;
use spring_redis::cache;
use spring_sea_orm::{DbConn, pagination::{Page, Pagination, PaginationExt}};
use std::{collections::HashMap, num::ParseIntError};

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

    pub async fn find_year_stats_for_category(
        &self,
        kp_id: i32,
    ) -> anyhow::Result<Vec<question_keypoint_stats::Model>> {
        QuestionKeyPointStats::find()
            .filter(question_keypoint_stats::Column::KeyPointId.eq(kp_id)) // Assuming 1 is the paper type for year stats
            .order_by_asc(question_keypoint_stats::Column::Year)
            .all(&self.db)
            .await
            .context("find_year_stats_for_category")
    }

    pub async fn find_key_point_by_path(
        &self,
        paper_type: i16,
        path: &str,
    ) -> anyhow::Result<Vec<KeyPointPath>> {
        let kp_ids = path
            .split(".")
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.parse::<i32>())
            .collect::<Result<Vec<i32>, ParseIntError>>()
            .context("invalid keypoint path format")?;

        let mut pid = 0;
        let mut kpp = vec![];
        for kp_id in kp_ids {
            let kps = self
                .find_key_point_by_pid_with_cache(paper_type, pid)
                .await?;
            pid = kp_id;
            kpp.push(KeyPointPath {
                kps,
                selected: kp_id,
            });
        }
        Ok(kpp)
    }

    #[cache("key_point:by_pid:{paper_type}:{key_point_id}")]
    pub async fn find_key_point_by_pid_with_cache(
        &self,
        paper_type: i16,
        key_point_id: i32,
    ) -> anyhow::Result<Vec<key_point::Model>> {
        self.find_key_point_by_pid(paper_type, key_point_id).await
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
            .order_by_asc(key_point::Column::Id)
            .all(&self.db)
            .await
            .with_context(|| format!("find_key_point_by_pid({key_point_id})"))
    }

    pub async fn find_qid_by_kp(
        &self,
        key_point_id: i32,
        year: Option<i16>,
        page: &Pagination,
    ) -> anyhow::Result<Page<i32>> {
        let mut condition = question_keypoint::Column::KeyPointId.eq(key_point_id);
        if let Some(y) = year {
            condition = condition.and(question_keypoint::Column::Year.eq(y))
        }
        QuestionKeyPoint::find()
            .select_only()
            .column(question_keypoint::Column::QuestionId)
            .filter(condition)
            .into_tuple()
            .page(&self.db, page)
            .await
            .with_context(|| format!("find_qid_by_kp({key_point_id}, {year:?})"))
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
