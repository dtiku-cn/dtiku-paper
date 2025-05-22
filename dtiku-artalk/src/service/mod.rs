pub mod proto {
    tonic::include_proto!("artalk");
}

use proto::artalk_service_server::{ArtalkService, ArtalkServiceServer};
use proto::{UserReq, UserResp};
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
        let identity: AuthIdentity = sqlx::query_as::<_, AuthIdentity>(
            r#"
            SELECT remote_uid, token, user_id
            FROM atk_auth_identities
            WHERE user_id = $1
            "#,
        )
        .bind(request.get_ref().user_id)
        .fetch_one(&self.db)
        .await
        .map_err(|e| Status::internal(format!("sqlx query failed:{e:?}")))?;
        Ok(tonic::Response::new(UserResp {
            user_id: identity.user_id,
            remote_uid: identity.remote_uid,
            token: identity.token,
        }))
    }
}

#[derive(Debug, FromRow)]
struct AuthIdentity {
    remote_uid: String,
    token: String,
    user_id: i32,
}
