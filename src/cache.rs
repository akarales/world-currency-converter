use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Duration, Utc};

#[derive(Clone, Debug)]
struct CacheEntry<T> {
    data: T,
    expires_at: DateTime<Utc>,
}

pub struct Cache<T> {
    store: Arc<RwLock<HashMap<String, CacheEntry<T>>>>,
    ttl: Duration,
    max_size: usize,
}

impl<T: Clone + Send + Sync + 'static> Cache<T> {
    pub fn new(ttl_minutes: i64, max_size: usize) -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            ttl: Duration::minutes(ttl_minutes),
            max_size,
        }
    }

    pub async fn get(&self, key: &str) -> Option<T> {
        let store = self.store.read().await;
        if let Some(entry) = store.get(key) {
            if entry.expires_at > Utc::now() {
                return Some(entry.data.clone());
            }
        }
        None
    }

    pub async fn set(&self, key: String, value: T) {
        let mut store = self.store.write().await;
        if store.len() >= self.max_size {
            return;
        }
        
        store.insert(
            key,
            CacheEntry {
                data: value,
                expires_at: Utc::now() + self.ttl,
            },
        );
    }

    pub async fn clear_expired(&self) {
        let mut store = self.store.write().await;
        store.retain(|_, entry| entry.expires_at > Utc::now());
    }
}

#[derive(Clone, Debug)]
pub struct ExchangeRateData {
    pub rate: f64,
    pub last_updated: DateTime<Utc>,
}

impl ExchangeRateData {
    pub fn new_cache() -> Cache<Self> {
        Cache::new(60, 1000) // 60 minutes TTL, 1000 max entries
    }
}