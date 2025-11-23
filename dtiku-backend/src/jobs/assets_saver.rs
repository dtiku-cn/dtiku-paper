use crate::plugins::embedding::Embedding;
use anyhow::Context as _;
use dtiku_base::model::{schedule_task, ScheduleTask};
use dtiku_paper::model::{assets, Assets};
use futures::future;
use sea_orm::{ActiveValue::Set, EntityTrait as _};
use serde_json::Value;
use spring::{plugin::service::Service, tracing};
use spring_opendal::Op;
use spring_sea_orm::DbConn;
use spring_stream::{
    extractor::{Component, Json},
    stream_listener,
};

const CLOUD_STORAGE: [&str; 3] = ["139", "115", "uc"];

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
    #[allow(unused)]
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

    async fn sync_save_assets(&mut self) -> anyhow::Result<()> {
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
            self.task = self.task.update_context(last_id, &self.db).await?;
        }
    }

    async fn write_to_storage(&self, a: &assets::Model) -> Result<(), anyhow::Error> {
        let storage_path = a.compute_storage_path();
        let img_url = Self::add_default_http(&a.src_url);
        let img_url = &img_url;
        let resp = reqwest::get(img_url)
            .await
            .with_context(|| format!("reqwest::get_img({img_url}) failed"))?;
        let body = resp
            .bytes()
            .await
            .with_context(|| format!("reqwest::get_img_body({img_url}) failed"))?;

        let futures = CLOUD_STORAGE.into_iter().map(|dir_prefix| {
            let dav = self.op.clone();
            let data = body.clone();
            let file_path = format!("{dir_prefix}/{storage_path}");
            async move {
                if !dav.exists(&file_path).await? {
                    let resp = dav
                        .write(&file_path, data)
                        .await
                        .with_context(|| format!("upload to {file_path} failed"))?;
                    tracing::debug!("upload ==> {resp:?}");
                } else {
                    tracing::info!("{file_path} exists");
                }
                Ok::<(), anyhow::Error>(())
            }
        });

        future::join_all(futures).await;

        Ok(())
    }

    fn add_default_http(url: &str) -> String {
        if url.starts_with("http://") || url.starts_with("https://") {
            url.to_string()
        } else {
            format!("http://{}", url)
        }
    }
}
