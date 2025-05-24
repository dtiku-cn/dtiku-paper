use crate::{
    plugins::grpc_client::{artalk::UserResp, Artalk},
    rpc::{self, artalk},
};
use anyhow::Context;
use dtiku_base::model::{user_info, UserInfo};
use sea_orm::ActiveModelTrait;
use sea_orm::ActiveValue::Set;
use spring::{plugin::service::Service, tracing};
use spring_sea_orm::DbConn;
use spring_web::axum::http;

#[derive(Debug, Clone, Service)]
pub struct UserService {
    #[inject(component)]
    db: DbConn,
    #[inject(component)]
    artalk: Artalk,
}

impl UserService {
    pub async fn auth_callback(
        &self,
        headers: http::HeaderMap,
        provider: String,
        raw_query: String,
    ) -> anyhow::Result<String> {
        let html = artalk::auth_callback(headers, &provider, &raw_query)
            .await
            .context("artalk callback failed")?;
        let token = substring_between(&html, "{ type: 'ATK_AUTH_CALLBACK', payload: '", "' },")
            .unwrap_or_default();
        Ok(token.to_string())
    }

    pub async fn get_user_detail(&self, user_id: i32) -> anyhow::Result<user_info::Model> {
        let u = UserInfo::find_user_by_id(&self.db, user_id)
            .await
            .context("get user detail failed")?;

        let u = match u {
            Some(u) => u,
            None => {
                let UserResp {
                    token, remote_uid, ..
                } = self.artalk.auth_identity(user_id).await?;
                let wechat_user = rpc::wechat_user_info(&token, &remote_uid)
                    .await
                    .with_context(|| format!("wechat_user_info({token},{remote_uid})"))?;

                user_info::ActiveModel {
                    id: Set(user_id),
                    wechat_id: Set(remote_uid),
                    gender: Set(wechat_user.sex == 1),
                    name: Set(wechat_user.nickname),
                    avatar: Set(wechat_user.headimgurl),
                    ..Default::default()
                }
                .insert(&self.db)
                .await
                .with_context(|| format!("insert user failed"))?
            }
        };
        Ok(u)
    }
}

fn substring_between<'a>(s: &'a str, start: &str, end: &str) -> Option<&'a str> {
    if let Some(start_pos) = s.find(start) {
        let rest = &s[start_pos + start.len()..];
        if let Some(end_pos) = rest.find(end) {
            return Some(&rest[..end_pos]);
        }
    }
    None
}
