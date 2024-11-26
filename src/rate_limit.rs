use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Duration, Utc};
use log::{debug, warn};

#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    pub requests: Vec<DateTime<Utc>>,
    pub daily_count: usize,
    pub last_reset: DateTime<Utc>,
}

pub struct RateLimiter {
    limits: Arc<RwLock<HashMap<String, RateLimitInfo>>>,
    daily_limit: usize,
    cleanup_interval: Duration,
    last_cleanup: Arc<RwLock<DateTime<Utc>>>,
}

impl RateLimiter {
    pub fn new(daily_limit: usize) -> Self {
        Self {
            limits: Arc::new(RwLock::new(HashMap::new())),
            daily_limit,
            cleanup_interval: Duration::minutes(5),
            last_cleanup: Arc::new(RwLock::new(Utc::now())),
        }
    }

    pub async fn check_rate_limit(&self, key: &str) -> bool {
        self.cleanup_if_needed().await;
        
        let mut limits = self.limits.write().await;
        let now = Utc::now();
        
        let info = limits.entry(key.to_string()).or_insert_with(|| RateLimitInfo {
            requests: Vec::new(),
            daily_count: 0,
            last_reset: now,
        });
        
        // Reset daily count if it's a new day
        if info.last_reset.date_naive() < now.date_naive() {
            info.daily_count = 0;
            info.last_reset = now;
            info.requests.clear();
        }
        
        // Check if we're under the daily limit
        if info.daily_count >= self.daily_limit {
            warn!("Rate limit exceeded for key: {}. Daily count: {}", key, info.daily_count);
            return false;
        }
        
        // Update counters
        info.requests.push(now);
        info.daily_count += 1;
        
        debug!("Rate limit check passed for key: {}. Daily count: {}/{}", 
            key, info.daily_count, self.daily_limit);
        true
    }

    async fn cleanup_if_needed(&self) {
        let mut last_cleanup = self.last_cleanup.write().await;
        let now = Utc::now();
        
        if now - *last_cleanup > self.cleanup_interval {
            let mut limits = self.limits.write().await;
            // Use checked date arithmetic for safety
            limits.retain(|_, info| {
                info.last_reset.date_naive() >= now.date_naive()
                    .pred_opt()
                    .unwrap_or_else(|| now.date_naive())
            });
            *last_cleanup = now;
        }
    }

    pub async fn get_remaining_requests(&self, key: &str) -> usize {
        let limits = self.limits.read().await;
        if let Some(info) = limits.get(key) {
            self.daily_limit.saturating_sub(info.daily_count)
        } else {
            self.daily_limit
        }
    }
}