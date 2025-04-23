use crate::config::embedding::EmbeddingConfig;
use anyhow::Context;
use derive_more::derive::{Deref, DerefMut};
use dtiku_ai::embedding_service_client::EmbeddingServiceClient;
use dtiku_ai::TextReq;
use spring::{app::AppBuilder, async_trait, config::ConfigRegistry, error::Result, plugin::Plugin};
use std::sync::Arc;
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
    }
}

#[derive(Debug, Clone, Deref, DerefMut)]
pub struct Embedding(EmbeddingServiceClient<Channel>);

impl Embedding {
    pub async fn text_embedding(&mut self, text: &str) -> Result<Vec<f32>> {
        let resp = self
            .0
            .text_embedding(TextReq { text: text.into() })
            .await
            .context("embedding service call failed")?;
        Ok(resp.into_inner().embedding)
    }
}
