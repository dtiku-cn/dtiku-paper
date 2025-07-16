pub mod proto {
    tonic::include_proto!("embedding");
}

use crate::{config::embedding::EmbeddingConfig, plugins::embedding::proto::BatchTextReq};
use anyhow::Context;
use derive_more::derive::{Deref, DerefMut};
use proto::{embedding_service_client::EmbeddingServiceClient, TextReq};
use spring::{
    app::AppBuilder,
    async_trait,
    config::ConfigRegistry,
    plugin::{MutableComponentRegistry, Plugin},
};
use tonic::transport::Channel;

pub struct EmbeddingPlugin;

#[async_trait]
impl Plugin for EmbeddingPlugin {
    async fn build(&self, app: &mut AppBuilder) {
        let embedding_config = app
            .get_config::<EmbeddingConfig>()
            .expect("load huggingface config failed");

        let client = EmbeddingServiceClient::connect(embedding_config.url)
            .await
            .expect("embedding service connect failed");

        app.add_component(Embedding(client));
    }
}

#[derive(Debug, Clone, Deref, DerefMut)]
pub struct Embedding(EmbeddingServiceClient<Channel>);

impl Embedding {
    pub async fn text_embedding<S: Into<String>>(&self, text: S) -> anyhow::Result<Vec<f32>> {
        let resp = self
            .0
            .clone()
            .text_embedding(TextReq { text: text.into() })
            .await
            .context("embedding service call failed")?;
        Ok(resp.into_inner().embedding)
    }

    pub async fn batch_text_embedding<S: Into<String> + Clone>(
        &self,
        texts: &[S],
    ) -> anyhow::Result<Vec<Vec<f32>>> {
        let batch_size = texts.len() as u32;
        let resp = self
            .0
            .clone()
            .batch_text_embedding(BatchTextReq {
                texts: texts.iter().cloned().map(Into::into).collect(),
                batch_size: batch_size.min(5), // 默认5条文本为一批
            })
            .await
            .context("embedding service call failed")?;
        Ok(resp
            .into_inner()
            .embeddings
            .into_iter()
            .map(|e| e.embedding)
            .collect())
    }
}
