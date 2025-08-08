use std::time::Duration;

use crate::config::embedding::EmbeddingConfig;
use anyhow::Context;
use itertools::Itertools;
use reqwest::header::HeaderMap;
use spring::{
    app::AppBuilder,
    async_trait,
    config::ConfigRegistry,
    plugin::{MutableComponentRegistry, Plugin},
};

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
            .read_timeout(Duration::from_secs(500)) // Set a longer timeout for embedding requests
            .build()
            .expect("create embedding client failed");

        app.add_component(Embedding {
            url: embedding_config.url,
            client,
        });
    }
}

#[derive(Debug, Clone)]
pub struct Embedding {
    url: String,
    client: reqwest::Client,
}

impl Embedding {
    pub async fn text_embedding<S: Into<String>>(&self, text: S) -> anyhow::Result<Vec<f32>> {
        let Self { url, client } = self;
        let text: String = text.into();
        let resp = client
            .post(format!("{url}/text_embedding"))
            .json(&text)
            .send()
            .await
            .context("embedding service text_embedding call failed")?;
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
        let Self { url, client } = self;
        let texts = texts
            .into_iter()
            .map(|t| Into::<String>::into(t.clone()))
            .collect_vec();
        let resp = client
            .post(format!("{url}/batch_text_embedding"))
            .json(&texts)
            .send()
            .await
            .context("embedding service batch_text_embedding call failed")?;
        let embeddings = resp
            .json()
            .await
            .context("parse embeddings response failed")?;
        Ok(embeddings)
    }
}
