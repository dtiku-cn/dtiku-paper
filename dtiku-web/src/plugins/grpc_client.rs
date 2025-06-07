pub mod ai {
    tonic::include_proto!("embedding");
}
pub mod artalk {
    tonic::include_proto!("artalk");
}

use super::GrpcClientConfig;
use ai::{embedding_service_client::EmbeddingServiceClient, TextReq};
use anyhow::Context;
use artalk::{artalk_service_client::ArtalkServiceClient, UserResp, VoteStats};
use derive_more::derive::{Deref, DerefMut};
use spring::{
    app::AppBuilder,
    async_trait,
    config::ConfigRegistry,
    error::Result,
    plugin::{MutableComponentRegistry, Plugin},
};
use std::time::Duration;
use tonic::transport::Channel;

pub struct GrpcClientPlugin;

#[async_trait]
impl Plugin for GrpcClientPlugin {
    async fn build(&self, app: &mut AppBuilder) {
        let grpc_config = app
            .get_config::<GrpcClientConfig>()
            .expect("load grpc config failed");

        let channel = Channel::from_shared(grpc_config.embedding_url)
            .expect("url is invalid")
            .keep_alive_while_idle(true)
            .keep_alive_timeout(Duration::from_secs(10))
            .http2_keep_alive_interval(Duration::from_secs(30))
            .connect()
            .await
            .expect("connect embedding server failed");

        let embedding_client = EmbeddingServiceClient::new(channel);

        let channel = Channel::from_shared(grpc_config.artalk_url)
            .expect("url is invalid")
            .keep_alive_while_idle(true)
            .keep_alive_timeout(Duration::from_secs(10))
            .http2_keep_alive_interval(Duration::from_secs(30))
            .connect()
            .await
            .expect("connect artalk server failed");

        let artalk_client = ArtalkServiceClient::new(channel);

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

    pub async fn vote_stats(&self, page_key: String) -> Result<VoteStats> {
        let resp = self
            .0
            .clone()
            .vote_stats(artalk::PageReq { page_key })
            .await
            .context("artalk service call failed")?;
        Ok(resp.into_inner())
    }

    pub async fn batch_vote_stats(&self, pages_key: Vec<String>) -> Result<Vec<VoteStats>> {
        let resp = self
            .0
            .clone()
            .batch_vote_stats(artalk::MultiPageReq { pages_key })
            .await
            .context("artalk service call failed")?;
        Ok(resp.into_inner().stats)
    }
}
