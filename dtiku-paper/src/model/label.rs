pub use super::_entities::label::*;
use anyhow::Context;
use sea_orm::{
    sea_query::OnConflict, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter,
};
use spring::{plugin::ComponentRegistry, tracing, App};
use spring_redis::{
    redis::{AsyncCommands, RedisError},
    Redis,
};

impl ActiveModel {
    pub async fn insert_on_conflict<C: ConnectionTrait>(self, db: &C) -> Result<Model, DbErr> {
        Entity::insert(self)
            .on_conflict(
                OnConflict::columns([Column::PaperType, Column::Pid, Column::Name])
                    .update_column(Column::ExamId)
                    .to_owned(),
            )
            .exec_with_returning(db)
            .await
    }
}

impl Entity {
    pub async fn find_by_id_with_cache<C>(db: &C, id: i32) -> anyhow::Result<Option<Model>>
    where
        C: ConnectionTrait,
    {
        let mut redis = App::global()
            .get_component::<Redis>()
            .expect("redis component not found");
        let cache_key = format!("label:{id}");
        let cached: Result<String, RedisError> = redis.get(&cache_key).await;
        let label = match cached {
            Ok(json) => Some(serde_json::from_str(&json).context("label json decode failed")?),
            Err(e) => {
                tracing::error!("cache error:{:?}", e);
                let label = Entity::find_by_id(id)
                    .one(db)
                    .await
                    .with_context(|| format!("Label::find_by_id({id}) failed"))?;
                if let Some(label) = &label {
                    let _: () = redis
                        .set(
                            &cache_key,
                            serde_json::to_string(label).context("label json encode failed")?,
                        )
                        .await
                        .unwrap_or_else(|e| tracing::error!("cache error:{:?}", e));
                }
                label
            }
        };

        Ok(label)
    }

    pub async fn find_all_by_pid<C>(db: &C, pid: i32) -> anyhow::Result<Vec<Model>>
    where
        C: ConnectionTrait,
    {
        Entity::find()
            .filter(Column::Pid.eq(pid))
            .all(db)
            .await
            .with_context(|| format!("find_by_pid({pid}) failed"))
    }

    pub async fn find_by_exam_id_and_paper_type_and_name<C, S>(
        db: &C,
        exam_id: i16,
        paper_type: i16,
        name: S,
    ) -> anyhow::Result<Option<Model>>
    where
        C: ConnectionTrait,
        S: Into<String>,
    {
        let name = name.into();
        Entity::find()
            .filter(
                Column::ExamId
                    .eq(exam_id)
                    .and(Column::PaperType.eq(paper_type).and(Column::Name.eq(&name))),
            )
            .one(db)
            .await
            .with_context(|| {
                format!(
                    "find_by_exam_id_and_paper_type_and_name({exam_id},{paper_type},{name}) failed"
                )
            })
    }
}
