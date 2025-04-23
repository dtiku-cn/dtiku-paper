use crate::config::hf_config::HfConfig;
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

pub struct EmbeddingPlugin;

#[async_trait]
impl Plugin for EmbeddingPlugin {
    async fn build(&self, app: &mut AppBuilder) {
        let hf_config = app
            .get_config::<HfConfig>()
            .expect("load huggingface config failed");
        let cache_dir = hf_config.cache_dir;
        let text_embedding = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::MultilingualE5Base)
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
