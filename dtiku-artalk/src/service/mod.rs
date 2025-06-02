pub mod proto {
    tonic::include_proto!("artalk");
}

use proto::artalk_service_server::{ArtalkService, ArtalkServiceServer};
use proto::{
    CommentReq, MultiPageReq, MultiVoteStats, PageReq, UserIdResp, UserReq, UserResp, VoteStats,
};
use spring::plugin::service::Service;
use spring_sqlx::{sqlx, ConnectPool};
use sqlx::FromRow;
use tonic::Status;

#[derive(Clone, Service)]
#[service(grpc = "ArtalkServiceServer")]
struct ArtalkServiceImpl {
    #[inject(component)]
    db: ConnectPool,
}

#[tonic::async_trait]
impl ArtalkService for ArtalkServiceImpl {
    async fn auth_identity(
        &self,
        request: tonic::Request<UserReq>,
    ) -> std::result::Result<tonic::Response<UserResp>, tonic::Status> {
        let identity = sqlx::query_as::<_, AuthIdentity>(
            r#"
            SELECT remote_uid, token, user_id
            FROM atk_auth_identities
            WHERE user_id = $1
            "#,
        )
        .bind(request.get_ref().user_id)
        .fetch_one(&self.db)
        .await
        .map_err(|e| Status::internal(format!("auth_identity sqlx query failed:{e:?}")))?;
        Ok(tonic::Response::new(UserResp {
            user_id: identity.user_id,
            remote_uid: identity.remote_uid,
            token: identity.token,
        }))
    }

    async fn comment_user(
        &self,
        request: tonic::Request<CommentReq>,
    ) -> std::result::Result<tonic::Response<UserIdResp>, tonic::Status> {
        let user_id = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT user_id
            FROM atk_comments
            WHERE id = $1
            "#,
        )
        .bind(request.get_ref().comment_id)
        .fetch_one(&self.db)
        .await
        .map_err(|e| Status::internal(format!("comment_user sqlx query failed:{e:?}")))?;

        Ok(tonic::Response::new(UserIdResp {
            user_id: user_id as i32,
        }))
    }

    async fn vote_stats(
        &self,
        request: tonic::Request<PageReq>,
    ) -> std::result::Result<tonic::Response<VoteStats>, tonic::Status> {
        let page = sqlx::query_as::<_, PageResult>(
            r#"
            SELECT key, vote_up, vote_down
            FROM atk_pages
            WHERE key = $1
            "#,
        )
        .bind(&request.get_ref().page_key)
        .fetch_one(&self.db)
        .await
        .map_err(|e| Status::internal(format!("vote_stats sqlx query failed:{e:?}")))?;

        Ok(tonic::Response::new(VoteStats {
            page_key: page.key,
            vote_up: page.vote_up,
            vote_down: page.vote_down,
        }))
    }

    async fn batch_vote_stats(
        &self,
        request: tonic::Request<MultiPageReq>,
    ) -> std::result::Result<tonic::Response<MultiVoteStats>, tonic::Status> {
        let page = sqlx::query_as::<_, PageResult>(
            r#"
            SELECT key, vote_up, vote_down
            FROM atk_pages
            WHERE key = any($1)
            "#,
        )
        .bind(&request.get_ref().pages_key)
        .fetch_all(&self.db)
        .await
        .map_err(|e| Status::internal(format!("batch_vote_stats sqlx query failed:{e:?}")))?;

        let stats = page
            .into_iter()
            .map(|p| VoteStats {
                page_key: p.key,
                vote_up: p.vote_up,
                vote_down: p.vote_down,
            })
            .collect();

        Ok(tonic::Response::new(MultiVoteStats { stats }))
    }
}

#[derive(Debug, FromRow)]
struct AuthIdentity {
    remote_uid: String,
    token: String,
    user_id: i32,
}

#[derive(Debug, FromRow)]
struct PageResult {
    key: String,
    vote_up: i64,
    vote_down: i64,
}
