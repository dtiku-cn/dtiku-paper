use derive_more::derive::Deref;
use fastembed::{
    EmbeddingModel, ImageEmbedding, ImageEmbeddingModel, ImageInitOptions, InitOptions,
    TextEmbedding,
};
use spring::{app::AppBuilder, async_trait, plugin::Plugin};
use std::sync::Arc;

pub struct EmbeddingPlugin;

#[async_trait]
impl Plugin for EmbeddingPlugin {
    async fn build(&self, app: &mut AppBuilder) {
        let text_embedding = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::ParaphraseMLMpnetBaseV2)
                .with_show_download_progress(true),
        )
        .expect("text embedding init failed");

        let image_embedding = ImageEmbedding::try_new(
            ImageInitOptions::new(ImageEmbeddingModel::Resnet50).with_show_download_progress(true),
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
