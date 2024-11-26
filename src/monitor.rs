use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Default, Serialize, Clone)]
pub struct UsageStats {
    pub total_requests: usize,
    pub successful_requests: usize,
    pub cache_hits: usize,
    pub api_calls: usize,
    pub errors: usize,
    pub last_reset: DateTime<Utc>,
}

pub struct UsageMonitor {
    stats: Arc<RwLock<UsageStats>>,
}

impl UsageMonitor {
    pub fn new() -> Self {
        Self {
            stats: Arc::new(RwLock::new(UsageStats {
                last_reset: Utc::now(),
                ..Default::default()
            })),
        }
    }

    pub async fn record_request(&self, cached: bool) {
        let mut stats = self.stats.write().await;
        stats.total_requests += 1;
        if cached {
            stats.cache_hits += 1;
        } else {
            stats.api_calls += 1;
        }
    }

    pub async fn record_error(&self) {
        let mut stats = self.stats.write().await;
        stats.errors += 1;
    }

    pub async fn get_stats(&self) -> UsageStats {
        let stats = self.stats.read().await;
        (*stats).clone()
    }

    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = UsageStats {
            last_reset: Utc::now(),
            ..Default::default()
        };
    }
}