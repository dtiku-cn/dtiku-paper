use crate::plugins::embedding::Embedding;
use anyhow::Context as _;
use dtiku_base::model::{schedule_task, ScheduleTask};
use dtiku_paper::model::Assets;
use sea_orm::{ActiveValue::Set, EntityTrait as _};
use serde_json::Value;
use spring::{plugin::service::Service, tracing};
use spring_opendal::Op;
use spring_sea_orm::DbConn;

#[derive(Debug, Service)]
#[service(prototype)]
pub struct AssetsSaveService {
    #[inject(component)]
    db: DbConn,
    #[inject(component)]
    embedding: Embedding,
    #[inject(component)]
    op: Op,
    task: schedule_task::Model,
}

impl AssetsSaveService {
    pub async fn start(&mut self) {
        self.sync_save_assets()
            .await
            .expect("sync save assets failed");

        let _ = ScheduleTask::update(schedule_task::ActiveModel {
            id: Set(self.task.id),
            version: Set(self.task.version + 1),
            active: Set(false),
            ..Default::default()
        })
        .exec(&self.db)
        .await
        .is_err_and(|e| {
            tracing::error!("update task error: {:?}", e);
            false
        });
    }

    async fn sync_save_assets(&self) -> anyhow::Result<()> {
        let mut last_id = match &self.task.context {
            Value::Number(last_id) => last_id.as_i64().unwrap_or_default() as i32,
            _ => 0,
        };
        tracing::warn!("sync_save_assets() started");
        let cloud_storage = ["139", "ilanzou", "pan.wo", "photo.baidu", "115", "uc"];
        loop {
            let assets = Assets::find_by_id_gt(&self.db, last_id).await?;
            if assets.is_empty() {
                tracing::warn!("sync_save_assets() finished");
                return Ok(());
            }
            for a in assets {
                let storage_path = a.compute_storage_path();
                let img_url = a.src_url;
                let resp = reqwest::get(&img_url)
                    .await
                    .with_context(|| format!("reqwest::get_img({img_url}) failed"))?;
                let body = resp
                    .bytes()
                    .await
                    .with_context(|| format!("reqwest::get_img_body({img_url}) failed"))?;
                for s in cloud_storage {
                    self.op
                        .write(&format!("{s}/{storage_path}"), body.clone())
                        .await
                        .with_context(|| format!("op.write({storage_path}) failed"))?;
                }
                last_id = a.id.max(last_id);
            }
        }
    }
}
