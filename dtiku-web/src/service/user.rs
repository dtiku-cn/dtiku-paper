use crate::{
    plugins::grpc_client::{artalk::UserResp, Artalk},
    rpc::{self, artalk},
};
use anyhow::Context;
use chrono::{Duration, Local};
use dtiku_base::model::{user_info, UserInfo};
use dtiku_pay::model::OrderLevel;
use sea_orm::ActiveModelTrait;
use sea_orm::ActiveValue::Set;
use spring::plugin::service::Service;
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

    pub async fn get_comment_user_avatar(&self, comment_id: i64) -> anyhow::Result<Option<String>> {
        let user_id = self.artalk.comment_user(comment_id).await?;
        Ok(UserInfo::find_user_by_id(&self.db, user_id)
            .await
            .context("get user detail failed")?
            .map(|u| u.avatar))
    }

    pub async fn get_user_detail(&self, user_id: i32) -> anyhow::Result<user_info::Model> {
        let u = UserInfo::find_user_by_id(&self.db, user_id)
            .await
            .context("get user detail failed")?;

        let mut u = match u {
            Some(u) => u,
            None => {
                let UserResp {
                    token, remote_uid, ..
                } = self.artalk.auth_identity(user_id).await?;
                let wechat_user = rpc::wechat::wechat_user_info(&token, &remote_uid)
                    .await
                    .with_context(|| format!("wechat_user_info({token},{remote_uid})"))?;

                user_info::ActiveModel {
                    id: Set(user_id),
                    wechat_id: Set(remote_uid),
                    name: Set(wechat_user.nickname),
                    avatar: Set(wechat_user.headimgurl),
                    ..Default::default()
                }
                .insert(&self.db)
                .await
                .with_context(|| format!("insert user failed"))?
            }
        };

        // 检查用户的创建时间和过期时间是否均早于2025-11-07
        // 如果是，则将过期时间延长到当前时间 + 7天
        // 这么做的目的是为了让老用户能使用体验卡的功能
        let cutoff_date = chrono::NaiveDate::from_ymd_opt(2025, 11, 7)
            .and_then(|date| date.and_hms_opt(0, 0, 0))
            .expect("invalid cutoff date");

        if u.created < cutoff_date && u.expired < cutoff_date {
            let now = Local::now().naive_local();
            let new_expired = now + Duration::days(7);

            let updated_user = user_info::ActiveModel {
                id: Set(user_id),
                expired: Set(new_expired),
                ..Default::default()
            }
            .update(&self.db)
            .await
            .context("update user expired date failed")?;

            u = updated_user;
        }

        Ok(u)
    }

    pub async fn confirm_user(
        &self,
        user_id: i32,
        order_level: OrderLevel,
    ) -> anyhow::Result<user_info::Model> {
        let now = Local::now().naive_local();
        let expires = now + Duration::days(order_level.days() as i64);
        user_info::ActiveModel {
            id: Set(user_id),
            expired: Set(expires),
            ..Default::default()
        }
        .update(&self.db)
        .await
        .with_context(|| format!("update user failed"))
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
