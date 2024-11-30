use crate::{
    cache::{RateCache, CountryCache},
    clients::HttpClient,
    currency_service::CurrencyService,
    errors::ServiceError,
    config::Config,
};
use std::{sync::Arc, time::Duration};
use log::info;

pub struct ServiceRegistry {
    pub currency_service: Arc<CurrencyService<HttpClient>>,
    pub rate_cache: Arc<RateCache>,
    pub country_cache: Arc<CountryCache>,
}

impl ServiceRegistry {
    pub fn new(config: &Config) -> Result<Self, ServiceError> {
        // Initialize caches
        let rate_cache = Arc::new(RateCache::new_rate_cache());
        let country_cache = Arc::new(CountryCache::new_country_cache());

        // Initialize HTTP client with configuration
        let http_client = HttpClient::with_timeouts(
            Duration::from_secs(config.api_settings.request_timeout_seconds),
            config.exchange_rate_api_key.clone(),
        ).map_err(|e| ServiceError::InitializationError(format!("Failed to create HTTP client: {}", e)))?;

        // Initialize currency service
        let currency_service = Arc::new(
            CurrencyService::new(
                http_client,
                Arc::clone(&rate_cache),
            )
        );

        info!("Service registry initialized with caches: rate_cache={} entries, country_cache={} entries",
            config.cache_settings.exchange_rate_max_entries,
            config.cache_settings.country_info_max_entries
        );

        Ok(Self {
            currency_service,
            rate_cache,
            country_cache,
        })
    }

    // Start background tasks (cache cleanup, etc.)
    pub async fn start_background_tasks(&self, config: &Config) {
        let rate_cache = Arc::clone(&self.rate_cache);
        let country_cache = Arc::clone(&self.country_cache);
        let cleanup_interval = Duration::from_secs(
            (config.cache_settings.cache_cleanup_interval_minutes * 60) as u64
        );

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(cleanup_interval).await;
                Self::cleanup_caches(&rate_cache, &country_cache).await;
            }
        });
    }

    async fn cleanup_caches(rate_cache: &RateCache, country_cache: &CountryCache) {
        // Clean up expired entries
        rate_cache.clear_expired().await;
        country_cache.clear_expired().await;

        // Log cache statistics
        let rate_stats = rate_cache.get_stats().await;
        let country_stats = country_cache.get_stats().await;

        info!("Cache stats - Rates: {}/{} entries (hit rate: {:.2}%), Countries: {}/{} entries (hit rate: {:.2}%)",
            rate_stats.size, rate_stats.max_size, rate_stats.hit_rate * 100.0,
            country_stats.size, country_stats.max_size, country_stats.hit_rate * 100.0
        );
    }

    // Graceful shutdown
    pub async fn shutdown(&self) {
        info!("Initiating service registry shutdown");
        
        // Log final cache statistics
        let rate_stats = self.rate_cache.get_stats().await;
        let country_stats = self.country_cache.get_stats().await;
        
        info!("Final cache statistics:");
        info!("Rate cache: {} hits, {} misses, {:.2}% hit rate",
            rate_stats.hits, rate_stats.misses, rate_stats.hit_rate * 100.0);
        info!("Country cache: {} hits, {} misses, {:.2}% hit rate",
            country_stats.hits, country_stats.misses, country_stats.hit_rate * 100.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tokio;

    #[tokio::test]
    async fn test_registry_creation() {
        // Ensure we have an API key for tests
        if env::var("EXCHANGE_RATE_API_KEY").is_err() {
            env::set_var("EXCHANGE_RATE_API_KEY", "test_key");
        }

        let config = Config::with_test_settings();
        let registry = ServiceRegistry::new(&config).unwrap();

        // Verify caches are initialized with correct sizes
        assert_eq!(
            registry.rate_cache.get_stats().await.max_size,
            1000  // Production value
        );
        assert_eq!(
            registry.country_cache.get_stats().await.max_size,
            500   // Production value
        );

        // Test basic functionality
        let cache_stats = registry.rate_cache.get_stats().await;
        assert_eq!(cache_stats.hits, 0);
        assert_eq!(cache_stats.misses, 0);
    }

    #[tokio::test]
    async fn test_cache_cleanup() {
        let config = Config::with_test_settings();
        let registry = ServiceRegistry::new(&config).unwrap();

        // Add some test data
        let test_rate_data = crate::cache::ExchangeRateData {
            rate: 1.0,
            last_updated: chrono::Utc::now(),
        };

        let mut test_currencies = std::collections::HashMap::new();
        test_currencies.insert(
            "USD".to_string(),
            crate::models::CurrencyInfo {
                name: "US Dollar".to_string(),
                symbol: "$".to_string(),
            },
        );

        let test_country_data = crate::models::CountryInfo {
            name: crate::models::CountryName {
                common: "Test".to_string(),
                official: "Test".to_string(),
                native_name: None,
            },
            currencies: Some(test_currencies),
        };

        registry.rate_cache.set("test_rate".to_string(), test_rate_data).await;
        registry.country_cache.set("test_country".to_string(), test_country_data).await;

        // Run cleanup
        ServiceRegistry::cleanup_caches(&registry.rate_cache, &registry.country_cache).await;

        // Verify stats
        let rate_stats = registry.rate_cache.get_stats().await;
        let country_stats = registry.country_cache.get_stats().await;

        assert!(rate_stats.size <= rate_stats.max_size);
        assert!(country_stats.size <= country_stats.max_size);
    }

    #[tokio::test]
    async fn test_background_tasks() {
        let config = Config::with_test_settings();
        let registry = ServiceRegistry::new(&config).unwrap();

        // Start background tasks with short interval for testing
        registry.start_background_tasks(&config).await;

        // Wait for one cleanup cycle
        tokio::time::sleep(Duration::from_secs(
            (config.cache_settings.cache_cleanup_interval_minutes * 60 + 1) as u64
        )).await;

        // Verify caches are still operational
        assert!(registry.rate_cache.get_stats().await.max_size > 0);
        assert!(registry.country_cache.get_stats().await.max_size > 0);
    }

    #[tokio::test]
    async fn test_shutdown() {
        let config = Config::with_test_settings();
        let registry = ServiceRegistry::new(&config).unwrap();

        // Add some test data
        let test_rate_data = crate::cache::ExchangeRateData {
            rate: 1.0,
            last_updated: chrono::Utc::now(),
        };

        registry.rate_cache.set("test_rate".to_string(), test_rate_data).await;

        // Perform shutdown
        registry.shutdown().await;

        // Verify caches are still accessible
        let rate_stats = registry.rate_cache.get_stats().await;
        assert_eq!(rate_stats.size, 1);
    }
}