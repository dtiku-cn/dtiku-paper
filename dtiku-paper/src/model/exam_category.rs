pub use super::_entities::exam_category::*;
use anyhow::Context;
use sea_orm::{
    sea_query::OnConflict, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter,
    QueryOrder,
};
use spring_redis::cache;

impl Entity {
    pub async fn find_children_by_pid<C>(db: &C, pid: i16) -> anyhow::Result<Vec<Model>>
    where
        C: ConnectionTrait,
    {
        Entity::find()
            .filter(Column::Pid.eq(pid))
            .order_by_asc(Column::Id)
            .all(db)
            .await
            .with_context(|| format!("exam_category::find_all_by_pid({pid}) failed"))
    }

    pub async fn find_children_by_pids<C>(db: &C, pids: Vec<i16>) -> anyhow::Result<Vec<Model>>
    where
        C: ConnectionTrait,
    {
        Entity::find()
            .filter(Column::Pid.is_in(pids))
            .all(db)
            .await
            .context("exam_category::find_all_by_pids() failed")
    }

    #[cache("exam_category:root_exam:{prefix}", expire = 86400)]
    pub async fn find_by_root_prefix<C>(db: &C, prefix: &str) -> anyhow::Result<Option<Model>>
    where
        C: ConnectionTrait,
    {
        Self::find_by_pid_prefix(db, 0, prefix).await
    }

    #[cache("exam_category:{pid}:{prefix}", expire = 86400)]
    pub async fn find_by_pid_prefix<C>(
        db: &C,
        pid: i16,
        prefix: &str,
    ) -> anyhow::Result<Option<Model>>
    where
        C: ConnectionTrait,
    {
        Entity::find()
            .filter(Column::Pid.eq(pid).and(Column::Prefix.eq(prefix)))
            .one(db)
            .await
            .with_context(|| format!("exam_category::find_by_prefix({prefix}) failed"))
    }

    pub async fn find_root_by_id<C>(db: &C, mut id: i16) -> anyhow::Result<Option<Model>>
    where
        C: ConnectionTrait,
    {
        if id <= 0 {
            return Ok(None);
        }

        loop {
            let model = Entity::find_by_id(id).one(db).await?;
            if let Some(model) = model {
                if model.pid == 0 {
                    return Ok(Some(model));
                }
                id = model.pid;
            } else {
                return Ok(model);
            }
        }
    }

    pub async fn find_leaf_by_pid<C>(db: &C, root_pid: i16) -> anyhow::Result<Vec<Model>>
    where
        C: ConnectionTrait,
    {
        let root = Entity::find_by_id(root_pid).one(db).await?;
        if root.is_none() {
            return Ok(vec![]);
        }

        let mut leaf_nodes = vec![];
        let mut stack = vec![root.unwrap()]; // 注意这里是 Model，不是 id

        while let Some(current) = stack.pop() {
            let children = Entity::find()
                .filter(Column::Pid.eq(current.id))
                .all(db)
                .await?;

            if children.is_empty() {
                leaf_nodes.push(current); // 是叶子节点
            } else {
                stack.extend(children); // 把 Model 推入栈
            }
        }

        Ok(leaf_nodes)
    }
}

impl ActiveModel {
    pub async fn insert_on_conflict<C: ConnectionTrait>(self, db: &C) -> Result<Model, DbErr> {
        Entity::insert(self)
            .on_conflict(
                OnConflict::columns([Column::FromTy, Column::Pid, Column::Prefix])
                    .update_column(Column::Name)
                    .to_owned(),
            )
            .exec_with_returning(db)
            .await
    }
}
