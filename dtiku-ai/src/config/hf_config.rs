use fastembed::{EmbeddingModel, ImageEmbeddingModel};
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use spring::config::Configurable;

#[serde_as]
#[derive(Debug, Deserialize, Configurable)]
#[config_prefix = "huggingface"]
pub struct HfConfig {
    #[serde(default = "default_cache_dir")]
    pub cache_dir: String,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(default = "default_text_model")]
    pub text_model: EmbeddingModel,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(default = "default_img_model")]
    pub img_model: ImageEmbeddingModel,
}

fn default_cache_dir() -> String {
    "../.hf-cache".to_string()
}

fn default_text_model() -> EmbeddingModel {
    EmbeddingModel::MultilingualE5Base
}

fn default_img_model() -> ImageEmbeddingModel {
    ImageEmbeddingModel::Resnet50
}
