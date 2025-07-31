use crate::config::embedding::EmbeddingConfig;
use anyhow::Context;
use axum::http::HeaderMap;
use derive_more::derive::{Deref, DerefMut};
use itertools::Itertools;
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

        let headers = HeaderMap::new();

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .expect("create embedding client failed");

        app.add_component(Embedding(client));
    }
}

#[derive(Debug, Clone, Deref, DerefMut)]
pub struct Embedding(reqwest::Client);

impl Embedding {
    pub async fn text_embedding<S: Into<String>>(&self, text: S) -> anyhow::Result<Vec<f32>> {
        let text: String = text.into();
        let resp = self
            .0
            .post("https://holmofy-dtiku-ai.hf.space/text_embedding")
            .json(&text)
            .send()
            .await
            .context("embedding service call failed")?;
        let embedding = resp
            .json()
            .await
            .context("parse embedding response failed")?;
        Ok(embedding)
    }

    pub async fn batch_text_embedding<S: Into<String> + Clone>(
        &self,
        texts: &[S],
    ) -> anyhow::Result<Vec<Vec<f32>>> {
        let texts = texts
            .into_iter()
            .map(|t| Into::<String>::into(t.clone()))
            .collect_vec();
        let resp = self
            .0
            .post("https://holmofy-dtiku-ai.hf.space/batch_text_embedding")
            .json(&texts)
            .send()
            .await
            .context("embedding service call failed")?;
        let embeddings = resp
            .json()
            .await
            .context("parse embedding response failed")?;
        Ok(embeddings)
    }
}
