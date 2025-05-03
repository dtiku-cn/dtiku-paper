use crate::config::hf_config::HfConfig;
use crate::service::embedding::{
    proto::embedding_service_server::EmbeddingServiceServer, EmbeddingServiceImpl,
};
use anyhow::Context;
use fastembed::{
    EmbeddingModel, ImageEmbedding, ImageEmbeddingModel, ImageInitOptions, InitOptions,
    TextEmbedding,
};
use ort::execution_providers::{
    CPUExecutionProvider, CUDAExecutionProvider, ROCmExecutionProvider, TensorRTExecutionProvider,
};
use spring::tracing::Level;
use spring::{app::AppBuilder, async_trait, config::ConfigRegistry, error::Result, plugin::Plugin};
use spring::{tracing, App};
use spring_opentelemetry::trace;
use spring_web::middleware::trace::{DefaultMakeSpan, DefaultOnRequest, TraceLayer};
use std::net::SocketAddr;
use std::sync::Arc;
use tonic::transport::Server;

pub struct EmbeddingPlugin;

#[async_trait]
impl Plugin for EmbeddingPlugin {
    async fn build(&self, app: &mut AppBuilder) {
        let hf_config = app
            .get_config::<HfConfig>()
            .expect("load huggingface config failed");

        app.add_scheduler(move |app: Arc<App>| Box::new(Self::schedule(app, hf_config)));
    }
}

impl EmbeddingPlugin {
    async fn schedule(_app: Arc<App>, hf_config: HfConfig) -> Result<String> {
        let cache_dir = hf_config.cache_dir;

        let execution_providers = vec![
            CUDAExecutionProvider::default().build(),
            TensorRTExecutionProvider::default().build(),
            ROCmExecutionProvider::default().build(),
            CPUExecutionProvider::default().build(),
        ];

        tracing::info!("load huggingface model");
        let text_embedding = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::MultilingualE5Base)
                .with_show_download_progress(true)
                .with_cache_dir(format!("{cache_dir}/sentence-transformers").into())
                .with_execution_providers(execution_providers.clone()),
        )
        .expect("text embedding init failed");

        let image_embedding = ImageEmbedding::try_new(
            ImageInitOptions::new(ImageEmbeddingModel::Resnet50)
                .with_show_download_progress(true)
                .with_cache_dir(format!("{cache_dir}/resnet").into())
                .with_execution_providers(execution_providers.clone()),
        )
        .expect("image embedding init failed");

        let addr = SocketAddr::new(hf_config.binding, hf_config.port);

        tracing::info!("tonic grpc service bind tcp listener: {}", addr);
        Server::builder()
            .layer(
                TraceLayer::new_for_grpc()
                    .make_span_with(DefaultMakeSpan::default().level(Level::INFO))
                    .on_request(DefaultOnRequest::default().level(Level::INFO)),
            )
            .layer(trace::GrpcLayer::server(Level::INFO))
            .add_service(EmbeddingServiceServer::new(EmbeddingServiceImpl {
                text_embedding,
                image_embedding,
            }))
            .serve(addr)
            .await
            .with_context(|| format!("bind tcp listener failed:{}", addr))?;

        Ok("embedding tonic server schedule finished".to_string())
    }
}

#[cfg(test)]
mod tests {
    use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
    use std::env;

    #[test]
    fn test_embed() {
        env::set_var("HF_ENDPOINT", "https://hf-mirror.com");
        let cache_dir = "/Users/holmofy/rust/dtiku-paper/.hf-cache";
        let text_embedding = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::MultilingualE5Base)
                .with_show_download_progress(true)
                .with_cache_dir(format!("{cache_dir}/sentence-transformers").into()),
        )
        .expect("text embedding init failed");

        let r = text_embedding.embed(vec!["hello world"], None);
        println!("{r:?}")
    }
}
