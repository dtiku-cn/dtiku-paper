use crate::rpc::artalk;
use anyhow::Context;
use dtiku_base::model::{user_info, UserInfo};
use spring::plugin::service::Service;
use spring_sea_orm::DbConn;

#[derive(Debug, Clone, Service)]
pub struct UserService {
    #[inject(component)]
    db: DbConn,
}

impl UserService {
    pub async fn auth_callback(
        &self,
        provider: String,
        raw_query: String,
    ) -> anyhow::Result<String> {
        let html = artalk::auth_callback(provider, raw_query)
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
                todo!()
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
