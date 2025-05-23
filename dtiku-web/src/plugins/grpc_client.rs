pub mod ai {
    tonic::include_proto!("embedding");
}
pub mod artalk {
    tonic::include_proto!("artalk");
}

use crate::config::embedding::EmbeddingConfig;
use anyhow::Context;
use derive_more::derive::{Deref, DerefMut};
use ai::{embedding_service_client::EmbeddingServiceClient, TextReq};
use spring::{
    app::AppBuilder,
    async_trait,
    config::ConfigRegistry,
    error::Result,
    plugin::{MutableComponentRegistry, Plugin},
};
use tonic::transport::Channel;

pub struct GrpcClientPlugin;

#[async_trait]
impl Plugin for GrpcClientPlugin {
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
