use crate::{plugins::DtikuConfig, rpc, views::AntiBotCapTemplate};
use anyhow::Context as _;
use askama::Template as _;
use spring::plugin::Service;
use spring_redis::{redis::AsyncCommands as _, Redis};
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
        let block_ip_key = format!("traffic:block_ip:{}", host);
        self.redis
            .hexists(block_ip_key, ip.to_string().as_str())
            .await
            .ok()
            .flatten()
            .unwrap_or(false)
    }

    pub fn gen_cap_template(&self) -> anyhow::Result<String> {
        let template = AntiBotCapTemplate {
            cap_site_key: self.config.cap_site_key.as_str(),
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
        let block_ip_key = format!("traffic:block_ip:{}", host);
        let _: usize = self
            .redis
            .hdel(block_ip_key, client_ip.to_string().as_str())
            .await
            .ok()
            .flatten()
            .unwrap_or(0);
    }
}
