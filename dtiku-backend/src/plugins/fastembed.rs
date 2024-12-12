use derive_more::derive::Deref;
use fastembed::{
    EmbeddingModel, ImageEmbedding, ImageEmbeddingModel, ImageInitOptions, InitOptions,
    TextEmbedding,
};
use spring::{
    app::AppBuilder,
    async_trait,
    config::ConfigRegistry,
    plugin::{MutableComponentRegistry as _, Plugin},
};
use std::sync::Arc;

use crate::config::hf_config::HfConfig;

pub struct EmbeddingPlugin;

#[async_trait]
impl Plugin for EmbeddingPlugin {
    async fn build(&self, app: &mut AppBuilder) {
        let hf_config = app
            .get_config::<HfConfig>()
            .expect("load huggingface config failed");
        let cache_dir = hf_config.cache_dir;
        let text_embedding = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::ParaphraseMLMiniLML12V2Q)
                .with_show_download_progress(true)
                .with_cache_dir(format!("{cache_dir}/sentence-transformers").into()),
        )
        .expect("text embedding init failed");

        let image_embedding = ImageEmbedding::try_new(
            ImageInitOptions::new(ImageEmbeddingModel::Resnet50)
                .with_show_download_progress(true)
                .with_cache_dir(format!("{cache_dir}/resnet").into()),
        )
        .expect("image embedding init failed");

        app.add_component(TxtEmbedding::new(text_embedding));
        app.add_component(ImgEmbedding::new(image_embedding));
    }
}

#[derive(Clone, Deref)]
pub struct ImgEmbedding(Arc<ImageEmbedding>);

impl ImgEmbedding {
    fn new(model: ImageEmbedding) -> Self {
        Self(Arc::new(model))
    }
}

#[derive(Clone, Deref)]
pub struct TxtEmbedding(Arc<TextEmbedding>);

impl TxtEmbedding {
    fn new(model: TextEmbedding) -> Self {
        Self(Arc::new(model))
    }
}
