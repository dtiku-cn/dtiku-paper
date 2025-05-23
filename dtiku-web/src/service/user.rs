use anyhow::Context;
use sea_orm::{DbConn, EntityTrait};
use spring::plugin::service::Service;

use crate::{model::UserInfo, rpc::artalk};

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
        let u = UserInfo::find_by_id(user_id)
            .one(&self.db)
            .await
            .context("get user detail failed")?;

        match u {
            Some(u)=>u,
            None=>{
                
            }
        }
        todo!()
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
