pub mod proto {
    tonic::include_proto!("embedding");
}

use crate::config::embedding::EmbeddingConfig;
use anyhow::Context;
use derive_more::derive::{Deref, DerefMut};
use proto::{embedding_service_client::EmbeddingServiceClient, TextReq};
use spring::{
    app::AppBuilder,
    async_trait,
    config::ConfigRegistry,
    error::Result,
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
    pub async fn text_embedding<S: Into<String>>(&self, text: S) -> Result<Vec<f32>> {
        let resp = self
            .0
            .clone()
            .text_embedding(TextReq { text: text.into() })
            .await
            .context("embedding service call failed")?;
        Ok(resp.into_inner().embedding)
    }
}
