use chrono::{DateTime, Utc};
use dashmap::DashMap;
use dtiku_base::service::system_config::SystemConfigService;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use spring::tracing;
use std::cmp::{max, min};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::time;

use crate::service::user::UserService;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Traffic {
    pub ip: String,
    pub user_agent: String,
    pub path: String,
    pub user_id: Option<i32>,
    pub access_time: DateTime<Utc>,
}

impl Traffic {
    pub fn new(
        ip: impl Into<String>,
        user_agent: impl Into<String>,
        path: impl Into<String>,
        user_id: Option<i32>,
    ) -> Self {
        Self {
            ip: ip.into(),
            user_agent: user_agent.into(),
            path: path.into(),
            user_id,
            access_time: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CountStats<T> {
    pub name: T,
    pub value: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrafficStats {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub ip_stats: Vec<CountStats<String>>,
    pub agent_stats: Vec<CountStats<String>>,
    pub spider_stats: Vec<CountStats<String>>,
    pub path_stats: Vec<CountStats<String>>,
    pub user_stats: Vec<CountStats<i32>>,
}

pub struct TrafficAnalysis {
    pub stats: Arc<Mutex<Option<TrafficStats>>>,
    traffics: Arc<Mutex<Vec<Traffic>>>,
    analysis: Arc<Mutex<Vec<Traffic>>>,
    system_config_service: SystemConfigService,
    user_service: UserService,
}

impl TrafficAnalysis {
    pub fn new(system_config_service: SystemConfigService, user_service: UserService) -> Self {
        Self {
            traffics: Arc::new(Mutex::new(Vec::with_capacity(1000))),
            analysis: Arc::new(Mutex::new(Vec::with_capacity(1000))),
            stats: Arc::new(Mutex::new(None)),
            system_config_service,
            user_service,
        }
    }

    pub async fn block_ip_pv_threshold(&self) -> u32 {
        self.system_config_service
            .block_ip_pv_threshold()
            .await
            .ok()
            .unwrap_or(100)
    }

    pub async fn seo_user_agents(&self) -> Vec<String> {
        self.system_config_service.parsed_seo_user_agents().await
    }

    pub fn record(&self, t: Traffic) {
        let mut traffics = self.traffics.lock().unwrap();
        traffics.push(t);
    }

    pub async fn scheduled(self: Arc<Self>) {
        let this = self.clone();
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(300)); // 5分钟
            loop {
                interval.tick().await;
                let analysis = this.swap();
                this.analysis(analysis).await;
            }
        });
    }

    fn swap(&self) -> Vec<Traffic> {
        let mut traffics = self.traffics.lock().unwrap();
        let mut analysis = self.analysis.lock().unwrap();
        std::mem::swap(&mut *traffics, &mut *analysis);
        analysis.drain(0..).collect()
    }

    async fn analysis(&self, analysis: Vec<Traffic>) {
        let block_ip_pv_threshold = self.block_ip_pv_threshold().await;

        let ip_accumulator = DashMap::<String, u32>::new();
        let agent_accumulator = DashMap::<String, u32>::new();
        let path_accumulator = DashMap::<String, u32>::new();
        let user_accumulator = DashMap::<i32, u32>::new();
        let mut ip_user_agent_map = HashMap::new();

        let mut start = None::<DateTime<Utc>>;
        let mut end = None::<DateTime<Utc>>;

        for traffic in analysis.iter() {
            let mut pv = ip_accumulator.entry(traffic.ip.clone()).or_insert(0);
            *pv += 1;
            if *pv > block_ip_pv_threshold {
                ip_user_agent_map.insert(traffic.ip.clone(), traffic.user_agent.clone());
            }

            *agent_accumulator
                .entry(traffic.user_agent.clone())
                .or_insert(0) += 1;
            *path_accumulator.entry(traffic.path.clone()).or_insert(0) += 1;
            if let Some(uid) = traffic.user_id {
                *user_accumulator.entry(uid).or_insert(0) += 1;
            }

            start = Some(start.map_or(traffic.access_time, |s| min(s, traffic.access_time)));
            end = Some(end.map_or(traffic.access_time, |e| max(e, traffic.access_time)));
        }

        let seo_user_agents = self.seo_user_agents().await;
        let is_seo_user_agent = |ua: &str| seo_user_agents.iter().any(|s| ua.contains(s));

        let order_by_count_desc = |map: &DashMap<String, u32>| {
            let mut v: Vec<_> = map
                .iter()
                .map(|entry| {
                    let (k, v) = (entry.key().clone(), *entry.value());
                    CountStats { name: k, value: v }
                })
                .collect();
            v.sort_by(|a, b| b.value.cmp(&a.value));
            v
        };

        let ip_stats = order_by_count_desc(&ip_accumulator);
        let mut agent_stats: Vec<_> = agent_accumulator
            .iter()
            .filter(|entry| !is_seo_user_agent(entry.key()))
            .map(|entry| CountStats {
                name: entry.key().clone(),
                value: *entry.value(),
            })
            .collect();
        agent_stats.sort_by(|a, b| b.value.cmp(&a.value));

        let mut spider_stats: Vec<_> = agent_accumulator
            .iter()
            .filter(|entry| is_seo_user_agent(entry.key()))
            .map(|entry| CountStats {
                name: entry.key().clone(),
                value: *entry.value(),
            })
            .collect();
        spider_stats.sort_by(|a, b| b.value.cmp(&a.value));

        let path_stats = order_by_count_desc(&path_accumulator);

        let mut user_stats: Vec<_> = user_accumulator
            .iter()
            .map(|entry| CountStats {
                name: *entry.key(),
                value: *entry.value(),
            })
            .collect();
        user_stats.sort_by(|a, b| b.value.cmp(&a.value));

        let start = start.unwrap_or_else(|| Utc::now());
        let end = end.unwrap_or_else(|| Utc::now());

        let stats = TrafficStats {
            start,
            end,
            ip_stats,
            agent_stats,
            spider_stats,
            path_stats,
            user_stats,
        };

        *self.stats.lock().unwrap() = Some(stats);

        // 5分钟内超过阈值，且 UA 不是 SEO 爬虫的 IP 视为可疑
        let black_ips: HashSet<String> = ip_accumulator
            .iter()
            .filter(|entry| *entry.value() > block_ip_pv_threshold)
            .map(|entry| entry.key().clone())
            .filter(|ip| match ip_user_agent_map.get(ip) {
                Some(ua) if !ua.is_empty() && !is_seo_user_agent(ua) => true,
                _ => false,
            })
            .collect();

        // 访问超过阈值、且不是 MASTER 用户的 user_id
        let user_ids: HashSet<i32> = user_accumulator
            .iter()
            .filter(|entry| *entry.value() > block_ip_pv_threshold)
            .map(|entry| *entry.key())
            //.filter(|user_id| *user_id != self.master_user_id) // 相当于 !User.MASTER_ID.equals(userId)
            .collect();

        // 这里替换为你自己的“写入黑名单 / 禁用用户”的持久化逻辑
        if !black_ips.is_empty() {
            tracing::warn!(
                "append {} ip to blacklist: {:?}",
                black_ips.len(),
                black_ips
            );
        }
        if !user_ids.is_empty() {
            tracing::warn!("block users: {:?}", user_ids);
        }
    }
}
