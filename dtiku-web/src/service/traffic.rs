use crate::{plugins::DtikuConfig, rpc, views::AntiBotCapTemplate};
use anyhow::Context as _;
use askama::Template as _;
use spring::plugin::Service;
use spring_redis::{
    redis::{AsyncCommands as _, Script},
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
        let block_ip_key = format!("traffic:block_ip:{}", host);

        let script = Script::new(
            r#"
local hash_key = KEYS[1]
local ip = ARGV[1]
local now = ARGV[2]

local json_val = redis.call("HGET", hash_key, ip)
if not json_val then
    return 0
end

local data = cjson.decode(json_val)
local block_until = data["block_until"]

if now >= block_until then
    redis.call("HDEL", hash_key, ip)
    return 0
end

return 1
"#,
        );

        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();

        let blocked: i32 = script
            .key(block_ip_key)
            .arg(ip.to_string())
            .arg(now)
            .invoke_async(&mut self.redis)
            .await
            .ok()
            .flatten()
            .unwrap_or(0);

        blocked == 1
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
