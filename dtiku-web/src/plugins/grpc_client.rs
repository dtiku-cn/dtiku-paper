pub mod ai {
    tonic::include_proto!("embedding");
}
pub mod artalk {
    tonic::include_proto!("artalk");
}

use super::GrpcClientConfig;
use ai::{embedding_service_client::EmbeddingServiceClient, TextReq};
use anyhow::Context;
use artalk::{artalk_service_client::ArtalkServiceClient, UserResp};
use derive_more::derive::{Deref, DerefMut};
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
        let grpc_config = app
            .get_config::<GrpcClientConfig>()
            .expect("load grpc config failed");

        let embedding_client = EmbeddingServiceClient::connect(grpc_config.embedding_url)
            .await
            .expect("embedding service connect failed");

        let artalk_client = ArtalkServiceClient::connect(grpc_config.artalk_url)
            .await
            .expect("artalk service connect failed");

        app.add_component(Embedding(embedding_client))
            .add_component(Artalk(artalk_client));
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

#[derive(Debug, Clone, Deref, DerefMut)]
pub struct Artalk(ArtalkServiceClient<Channel>);

impl Artalk {
    pub async fn auth_identity(&self, user_id: i32) -> Result<UserResp> {
        let resp = self
            .0
            .clone()
            .auth_identity(artalk::UserReq { user_id })
            .await
            .context("artalk service call failed")?;
        Ok(resp.into_inner())
    }

    pub async fn comment_user(&self, comment_id: i64) -> Result<i32> {
        let resp = self
            .0
            .clone()
            .comment_user(artalk::CommentReq { comment_id })
            .await
            .context("artalk service call failed")?;
        Ok(resp.into_inner().user_id)
    }
}
