use crate::models::CountryInfo;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Duration, Utc};
use log::debug;

// Define ExchangeRateData here since it's cache-specific
#[derive(Clone, Debug)]
pub struct ExchangeRateData {
    pub rate: f64,
    pub last_updated: DateTime<Utc>,
}

#[derive(Clone, Debug)]
struct CacheEntry<T> {
    data: T,
    expires_at: DateTime<Utc>,
}

pub struct Cache<T> {
    store: Arc<RwLock<HashMap<String, CacheEntry<T>>>>,
    ttl: Duration,
    max_size: usize,
    hits: Arc<RwLock<u64>>,
    misses: Arc<RwLock<u64>>,
}

impl<T: Clone + Send + Sync + 'static> Cache<T> {
    pub fn new(ttl_minutes: i64, max_size: usize) -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            ttl: Duration::minutes(ttl_minutes),
            max_size,
            hits: Arc::new(RwLock::new(0)),
            misses: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn get(&self, key: &str) -> Option<T> {
        let store = self.store.read().await;
        if let Some(entry) = store.get(key) {
            if entry.expires_at > Utc::now() {
                let mut hits = self.hits.write().await;
                *hits += 1;
                debug!("Cache hit for key: {}", key);
                return Some(entry.data.clone());
            }
        }
        let mut misses = self.misses.write().await;
        *misses += 1;
        debug!("Cache miss for key: {}", key);
        None
    }

    pub async fn set(&self, key: String, value: T) {
        let mut store = self.store.write().await;
        
        // If cache is at max size, remove oldest entry
        if store.len() >= self.max_size {
            debug!("Cache at max size ({}), removing oldest entry", self.max_size);
            if let Some(oldest_key) = store
                .iter()
                .min_by_key(|(_, entry)| entry.expires_at)
                .map(|(k, _)| k.clone())
            {
                store.remove(&oldest_key);
            }
        }
        
        store.insert(
            key.clone(),
            CacheEntry {
                data: value,
                expires_at: Utc::now() + self.ttl,
            },
        );
        debug!("Cached data for key: {}", key);
    }

    pub async fn clear_expired(&self) {
        let mut store = self.store.write().await;
        let now = Utc::now();
        let initial_size = store.len();
        store.retain(|_, entry| entry.expires_at > now);
        let removed = initial_size - store.len();
        if removed > 0 {
            debug!("Cleared {} expired cache entries", removed);
        }
    }

    pub async fn get_stats(&self) -> CacheStats {
        let hits = *self.hits.read().await;
        let misses = *self.misses.read().await;
        let total = hits + misses;
        let hit_rate = if total > 0 {
            hits as f64 / total as f64
        } else {
            0.0
        };

        CacheStats {
            size: self.store.read().await.len(),
            max_size: self.max_size,
            hits,
            misses,
            hit_rate,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub size: usize,
    pub max_size: usize,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
}

// Specific cache types
pub type CountryCache = Cache<CountryInfo>;
pub type RateCache = Cache<ExchangeRateData>;

impl CountryCache {
    pub fn new_country_cache() -> Self {
        // Cache country info for 24 hours since it rarely changes
        Cache::new(24 * 60, 500)
    }
}

impl RateCache {
    pub fn new_rate_cache() -> Self {
        // Cache exchange rates for 1 hour based on API update frequency
        Cache::new(60, 1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CountryName, CurrencyInfo};
    use std::collections::HashMap;

    pub fn create_test_country_info(name: &str) -> CountryInfo {
        let mut currencies = HashMap::new();
        currencies.insert(
            "USD".to_string(),
            CurrencyInfo {
                name: "US Dollar".to_string(),
                symbol: "$".to_string(),
            },
        );

        CountryInfo {
            name: CountryName {
                common: name.to_string(),
                official: format!("Official {}", name),
                native_name: None,
            },
            currencies: Some(currencies),
        }
    }

    #[tokio::test]
    async fn test_cache_basic_operations() {
        let cache = CountryCache::new_country_cache();
        let country = create_test_country_info("Test Country");
        
        // Test set and get
        cache.set("test".to_string(), country.clone()).await;
        let result = cache.get("test").await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().name.common, "Test Country");

        // Test non-existent key
        let result = cache.get("nonexistent").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let cache = Cache::<CountryInfo>::new(0, 10); // 0 minutes TTL for testing
        let country = create_test_country_info("Test Country");
        
        cache.set("test".to_string(), country.clone()).await;
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        
        let result = cache.get("test").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_max_size() {
        let cache = Cache::<CountryInfo>::new(60, 2); // Max size of 2
        let country1 = create_test_country_info("Country 1");
        let country2 = create_test_country_info("Country 2");
        let country3 = create_test_country_info("Country 3");

        cache.set("key1".to_string(), country1).await;
        cache.set("key2".to_string(), country2).await;
        cache.set("key3".to_string(), country3).await;

        let stats = cache.get_stats().await;
        assert_eq!(stats.size, 2);
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let cache = CountryCache::new_country_cache();
        let country = create_test_country_info("Test Country");

        // Create some cache activity
        cache.set("test".to_string(), country.clone()).await;
        cache.get("test").await;
        cache.get("nonexistent").await;

        let stats = cache.get_stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hit_rate, 0.5);
    }

    #[tokio::test]
    async fn test_clear_expired() {
        let cache = Cache::<CountryInfo>::new(0, 10); // 0 minutes TTL
        let country = create_test_country_info("Test Country");
        
        cache.set("test".to_string(), country.clone()).await;
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        
        cache.clear_expired().await;
        let stats = cache.get_stats().await;
        assert_eq!(stats.size, 0);
    }
}