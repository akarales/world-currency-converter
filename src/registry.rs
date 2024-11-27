use crate::{
    cache::{Cache, ExchangeRateData},
    clients::HttpClient,
    currency_service::CurrencyService,
    errors::ServiceError,
    config::Config,
};
use std::{sync::Arc, time::Duration};

pub struct ServiceRegistry {
    pub currency_service: Arc<CurrencyService<HttpClient>>,
    pub cache: Arc<Cache<ExchangeRateData>>,
}

impl ServiceRegistry {
    pub fn new(config: &Config) -> Result<Self, ServiceError> {
        // Initialize cache
        let cache = Arc::new(Cache::new(
            config.cache_settings.exchange_rate_ttl_minutes,
            1000 // max entries
        ));

        // Initialize HTTP client
        let http_client = HttpClient::with_timeouts(
            Duration::from_secs(30),
            config.exchange_rate_api_key.clone()
        )?;

        // Initialize currency service
        let currency_service = Arc::new(
            CurrencyService::new(
                http_client,
                Arc::clone(&cache)
            )
        );

        Ok(Self {
            currency_service,
            cache,
        })
    }

    // Add cleanup method for graceful shutdown
    pub async fn cleanup(&self) {
        // Add any cleanup logic here
        // For example: closing connections, flushing caches, etc.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, CacheSettings, RateLimitSettings};

    #[tokio::test]
    async fn test_registry_creation() {
        let config = Config {
            exchange_rate_api_key: "test_key".to_string(),
            cache_settings: CacheSettings {
                exchange_rate_ttl_minutes: 60,
                country_info_ttl_minutes: 1440,
                cache_cleanup_interval_minutes: 5,
            },
            rate_limit_settings: RateLimitSettings {
                requests_per_day: 1000,
                window_size_minutes: 1440,
            },
        };

        let registry = ServiceRegistry::new(&config).unwrap();
        assert!(Arc::strong_count(&registry.cache) >= 2); // At least two references: registry and service
    }
}