use crate::{plugins::DtikuConfig, rpc, views::AntiBotCapTemplate};
use anyhow::Context as _;
use askama::Template as _;
use spring::plugin::Service;
use spring_redis::{
    redis::AsyncCommands as _,
    Redis,
};
use std::net::IpAddr;

#[derive(Clone, Service)]
pub struct TrafficService {
    #[inject(component)]
    redis: Redis,
    #[inject(config)]
    config: DtikuConfig,
}

impl TrafficService {
    pub async fn is_block_ip(&mut self, host: &str, ip: IpAddr) -> bool {
        // 新的存储结构：独立 string key，格式为 traffic:block_ip:{host}:{ip}
        // Redis 会自动根据 TTL 过期，无需手动检查 block_until
        let block_ip_key = format!("traffic:block_ip:{}:{}", host, ip);

        let exists: bool = self
            .redis
            .exists(block_ip_key)
            .await
            .ok()
            .unwrap_or(false);

        exists
    }

    pub fn gen_cap_template(&self) -> anyhow::Result<String> {
        let template = AntiBotCapTemplate {
            cap_site_key: self.config.cap_site_key.as_str(),
            cap_custom_wasm_url: &self.config.cap_custom_wasm_url,
        };
        template.render().context("generate cap template failed")
    }

    pub async fn verify_token(&self, token: &str) -> anyhow::Result<bool> {
        rpc::cap::site_verify(
            self.config.cap_site_key.as_str(),
            self.config.cap_secret_key.as_str(),
            token,
        )
        .await
    }

    pub async fn unblock_ip(&mut self, host: &str, client_ip: IpAddr) {
        // 新的存储结构：独立 string key，直接删除 key 即可
        let block_ip_key = format!("traffic:block_ip:{}:{}", host, client_ip);
        let _: bool = self
            .redis
            .del(block_ip_key)
            .await
            .ok()
            .unwrap_or(false);
    }
}
