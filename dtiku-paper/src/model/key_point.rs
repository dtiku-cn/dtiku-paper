pub use super::_entities::key_point::*;
use crate::util;
use anyhow::Context;
use sea_orm::{
    sea_query::OnConflict, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter,
    QueryOrder,
};
use spring_redis::cache;

impl Entity {
    #[cache("keypoint:{id}", expire = 86400)]
    pub async fn find_by_id_with_cache<C: ConnectionTrait>(
        db: &C,
        id: i32,
    ) -> anyhow::Result<Option<Model>> {
        Entity::find_by_id(id)
            .one(db)
            .await
            .with_context(|| format!("KeyPoint::find_by_id_with_cache({id}) failed"))
    }

    pub async fn find_by_pid<C: ConnectionTrait>(
        db: &C,
        paper_type: i16,
        pid: i32,
    ) -> anyhow::Result<Vec<Model>> {
        Entity::find()
            .filter(Column::Pid.eq(pid).and(Column::PaperType.eq(paper_type)))
            .order_by_asc(Column::Id)
            .all(db)
            .await
            .with_context(|| format!("key_point::find_by_pid({paper_type},{pid}) failed"))
    }

    pub async fn find_by_pid_and_name<C: ConnectionTrait>(
        db: &C,
        paper_type: i16,
        pid: i32,
        name: &str,
    ) -> anyhow::Result<Option<Model>> {
        Entity::find()
            .filter(
                Column::Pid
                    .eq(pid)
                    .and(Column::PaperType.eq(paper_type))
                    .and(Column::Name.eq(name)),
            )
            .one(db)
            .await
            .with_context(|| {
                format!("key_point::find_by_pid_and_name({paper_type},{pid},{name}) failed")
            })
    }

    pub async fn find_by_paper_type<C: ConnectionTrait>(
        db: &C,
        paper_type: i16,
    ) -> Result<Vec<Model>, DbErr> {
        Entity::find()
            .filter(Column::PaperType.eq(paper_type))
            .order_by_asc(Column::Id)
            .all(db)
            .await
    }

    pub async fn find_by_paper_type_and_name<C: ConnectionTrait>(
        db: &C,
        paper_type: i16,
        name: &str,
    ) -> Result<Option<Model>, DbErr> {
        Entity::find()
            .filter(Column::PaperType.eq(paper_type).and(Column::Name.eq(name)))
            .one(db)
            .await
    }

    pub async fn query_common_keypoint_path<C: ConnectionTrait>(
        db: &C,
        keypoint_ids: &[i32],
    ) -> anyhow::Result<Option<String>> {
        let mut paths = vec![];
        for kp_id in keypoint_ids {
            let path = Self::query_keypoint_path(db, *kp_id).await?;
            if let Some(path) = path {
                paths.push(path);
            }
        }
        let paths: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
        let common_prefix = util::str::common_prefix_all(&paths);
        Ok(Some(common_prefix.trim_matches('.').to_string()))
    }

    pub async fn query_keypoint_path<C: ConnectionTrait>(
        db: &C,
        mut keypoint_id: i32,
    ) -> anyhow::Result<Option<String>> {
        let mut path = keypoint_id.to_string();
        loop {
            let kp = Self::find_by_id_with_cache(db, keypoint_id).await?;
            if let Some(kp) = kp {
                if kp.pid == 0 {
                    return Ok(Some(path));
                } else {
                    keypoint_id = kp.pid;
                    path.insert_str(0, &format!("{keypoint_id}."));
                }
            } else {
                return Ok(None);
            }
        }
    }
}

impl ActiveModel {
    pub async fn insert_on_conflict<C: ConnectionTrait>(self, db: &C) -> Result<Model, DbErr> {
        Entity::insert(self)
            .on_conflict(
                OnConflict::columns([Column::PaperType, Column::Pid, Column::Name])
                    .update_columns([Column::ExamId])
                    .to_owned(),
            )
            .exec_with_returning(db)
            .await
    }
}
