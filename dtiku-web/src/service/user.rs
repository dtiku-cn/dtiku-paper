use crate::{
    plugins::grpc_client::{artalk::UserResp, Artalk},
    plugins::AuthConfig,
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
use spring_redis::Redis;
use spring_web::axum::http;

#[derive(Clone, Service)]
pub struct UserService {
    #[inject(component)]
    db: DbConn,
    #[inject(component)]
    artalk: Artalk,
    #[inject(config)]
    auth_config: AuthConfig,
    #[inject(component)]
    redis: Redis,
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

    /// 根据微信 openid 查找或创建用户
    pub async fn find_or_create_wechat_user(
        &self,
        wechat_user: &crate::rpc::wechat::WechatMpUser,
    ) -> anyhow::Result<user_info::Model> {
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

        // 先尝试根据 wechat_id 查找用户
        let existing_user = user_info::Entity::find()
            .filter(user_info::Column::WechatId.eq(&wechat_user.openid))
            .one(&self.db)
            .await
            .context("查询用户失败")?;

        if let Some(user) = existing_user {
            // 如果用户已存在，更新头像和昵称
            let mut active_user: user_info::ActiveModel = user.into();
            active_user.name = Set(wechat_user.nickname.clone());
            active_user.avatar = Set(wechat_user.headimgurl.clone());
            
            let updated_user = active_user
                .update(&self.db)
                .await
                .context("更新用户信息失败")?;
            
            Ok(updated_user)
        } else {
            // 创建新用户
            let new_user = user_info::ActiveModel {
                wechat_id: Set(wechat_user.openid.clone()),
                name: Set(wechat_user.nickname.clone()),
                avatar: Set(wechat_user.headimgurl.clone()),
                ..Default::default()
            };

            let user = new_user
                .insert(&self.db)
                .await
                .context("创建用户失败")?;

            Ok(user)
        }
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

    /// 获取微信公众号 access_token（带缓存）
    pub async fn get_wechat_access_token(&self) -> anyhow::Result<String> {
        use spring_redis::redis::AsyncCommands;

        const CACHE_KEY: &str = "wechat:mp:access_token";
        const TOKEN_EXPIRE_MARGIN: i64 = 300; // 提前 5 分钟过期
        
        // 先尝试从缓存获取
        if let Some(token) = self.redis.clone().get::<_, Option<String>>(CACHE_KEY).await? {
            return Ok(token);
        }

        // 缓存未命中，从微信服务器获取
        let response = rpc::wechat::get_access_token(
            "client_credential",
            &self.auth_config.wechat_mp_app_id,
            &self.auth_config.wechat_mp_app_secret,
        )
        .await
        .context("获取 access_token 失败")?;

        // 检查微信 API 错误
        Self::check_wechat_error(response.errcode, response.errmsg.as_deref())?;

        let token = response
            .access_token
            .ok_or_else(|| anyhow::anyhow!("access_token 为空"))?;
        
        // 缓存 token（提前 5 分钟过期以确保安全）
        let expires_in = (response.expires_in.unwrap_or(7200) - TOKEN_EXPIRE_MARGIN).max(60);
        self.redis
            .clone()
            .set_ex(CACHE_KEY, &token, expires_in as u64)
            .await?;

        Ok(token)
    }

    /// 创建微信登录二维码
    pub async fn create_wechat_login_qrcode(&self, scene_id: &str) -> anyhow::Result<String> {
        let access_token = self.get_wechat_access_token().await?;
        
        // 使用字符串场景值创建临时二维码
        let request = rpc::wechat::CreateQrcodeRequest::new_temp_str(
            scene_id.to_string(),
            600, // 10 分钟有效期
        );

        let response = rpc::wechat::create_qrcode(&access_token, request)
            .await
            .context("创建二维码失败")?;

        // 检查微信 API 错误
        Self::check_wechat_error(response.errcode, response.errmsg.as_deref())?;

        response
            .ticket
            .ok_or_else(|| anyhow::anyhow!("ticket 为空"))
    }

    /// 获取微信用户信息
    pub async fn get_wechat_user_info(&self, openid: &str) -> anyhow::Result<rpc::wechat::WechatMpUser> {
        let access_token = self.get_wechat_access_token().await?;
        
        let user_info = rpc::wechat::get_user_info(&access_token, openid, "zh_CN")
            .await
            .context("获取用户信息失败")?;

        // 检查微信 API 错误
        Self::check_wechat_error(user_info.errcode, user_info.errmsg.as_deref())?;

        Ok(user_info)
    }

    /// 检查微信 API 错误响应
    fn check_wechat_error(errcode: Option<i32>, errmsg: Option<&str>) -> anyhow::Result<()> {
        if let Some(code) = errcode {
            if code != 0 {
                anyhow::bail!(
                    "微信 API 错误 [{}]: {}",
                    code,
                    errmsg.unwrap_or("未知错误")
                );
            }
        }
        Ok(())
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
