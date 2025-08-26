use crate::plugins::embedding::Embedding;
use anyhow::Context as _;
use dtiku_base::model::{schedule_task, ScheduleTask};
use dtiku_paper::model::{assets, Assets};
use sea_orm::{ActiveValue::Set, EntityTrait as _};
use serde_json::Value;
use spring::{plugin::service::Service, tracing};
use spring_opendal::Op;
use spring_sea_orm::DbConn;
use spring_stream::{
    extractor::{Component, Json},
    stream_listener,
};

const CLOUD_STORAGE: [&str; 6] = ["139", "ilanzou", "pan.wo", "photo.baidu", "115", "uc"];

#[stream_listener("assets")]
async fn save_assets_in_realtime(
    Component(ass): Component<AssetsSaveService>,
    Json(assets): Json<assets::Model>,
) {
    if let Err(e) = ass.write_to_storage(&assets).await {
        tracing::error!("assets write to storage failed: {e:?}");
    }
}

#[derive(Debug, Clone, Service)]
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
        loop {
            let assets = Assets::find_by_id_gt(&self.db, last_id).await?;
            if assets.is_empty() {
                tracing::warn!("sync_save_assets() finished");
                return Ok(());
            }
            for a in assets {
                self.write_to_storage(&a).await?;
                last_id = a.id.max(last_id);
            }
        }
    }

    async fn write_to_storage(&self, a: &assets::Model) -> Result<(), anyhow::Error> {
        let storage_path = a.compute_storage_path();
        let img_url = &a.src_url;
        let resp = reqwest::get(img_url)
            .await
            .with_context(|| format!("reqwest::get_img({img_url}) failed"))?;
        let body = resp
            .bytes()
            .await
            .with_context(|| format!("reqwest::get_img_body({img_url}) failed"))?;
        Ok(for s in CLOUD_STORAGE {
            if let Err(e) = self
                .op
                .write(&format!("{s}/{storage_path}"), body.clone())
                .await
            {
                tracing::error!("save asset failed: {e:?}");
            }
        })
    }
}
