# World Currency Converter Upgrade Plan

This document outlines the planned improvements and upgrades for the Currency Converter service. Each section details specific changes, benefits, and implementation examples.

## 1. Architecture Improvements

### 1.1 Dependency Injection

**Current Issue:** Services have hard-coded dependencies, making testing and modifications difficult.

**Solution:** Implement proper dependency injection pattern.

```rust
// New trait definitions
pub trait CurrencyClient {
    async fn get_exchange_rate(&self, from: &str, to: &str) -> Result<RateResponse, Error>;
    async fn get_country_info(&self, country: &str) -> Result<CountryInfo, Error>;
}

// Updated service structure
pub struct CurrencyService<C: CurrencyClient, R: RateLimit> {
    client: C,
    rate_limiter: R,
    cache: Arc<Cache>,
}

// Implementation
impl<C: CurrencyClient, R: RateLimit> CurrencyService<C, R> {
    pub fn new(client: C, rate_limiter: R, cache: Arc<Cache>) -> Self {
        Self {
            client,
            rate_limiter,
            cache,
        }
    }
}

// Mock implementation for testing
#[cfg(test)]
pub struct MockCurrencyClient {
    // Mock state
}

#[cfg(test)]
impl CurrencyClient for MockCurrencyClient {
    async fn get_exchange_rate(&self, from: &str, to: &str) -> Result<RateResponse, Error> {
        // Mock implementation
        Ok(RateResponse {
            rate: 1.0,
            timestamp: Utc::now(),
        })
    }
}
```

### 1.2 Error Handling

**Current Issue:** Generic error type doesn't provide enough context.

**Solution:** Implement domain-specific error types.

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Country not found: {0}")]
    CountryNotFound(String),
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("External API error: {0}")]
    ExternalApiError(#[from] reqwest::Error),
    
    #[error("Cache error: {0}")]
    CacheError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

impl From<ServiceError> for actix_web::Error {
    fn from(err: ServiceError) -> Self {
        match err {
            ServiceError::RateLimitExceeded => 
                HttpResponse::TooManyRequests().json(ErrorResponse {
                    error: "Rate limit exceeded".to_string(),
                    code: "RATE_LIMIT_EXCEEDED",
                }).into(),
            ServiceError::CountryNotFound(country) => 
                HttpResponse::NotFound().json(ErrorResponse {
                    error: format!("Country not found: {}", country),
                    code: "COUNTRY_NOT_FOUND",
                }).into(),
            _ => HttpResponse::InternalServerError().into(),
        }
    }
}
```

### 1.3 Service Registry

**Current Issue:** Service initialization and management is scattered.

**Solution:** Implement a central service registry.

```rust
pub struct ServiceRegistry {
    currency_service: Arc<CurrencyService>,
    rate_limiter: Arc<RateLimiter>,
    cache: Arc<Cache>,
    monitor: Arc<Monitor>,
}

impl ServiceRegistry {
    pub fn new(config: Config) -> Result<Self, ServiceError> {
        let cache = Arc::new(Cache::new(config.cache_settings));
        let rate_limiter = Arc::new(RateLimiter::new(config.rate_limit_settings));
        let client = create_http_client(&config)?;
        
        let currency_service = Arc::new(
            CurrencyService::new(
                client,
                Arc::clone(&rate_limiter),
                Arc::clone(&cache),
            )
        );

        Ok(Self {
            currency_service,
            rate_limiter,
            cache,
            monitor: Arc::new(Monitor::new()),
        })
    }

    pub fn currency_service(&self) -> &CurrencyService {
        &self.currency_service
    }
}
```

## 2. Performance Optimizations

### 2.1 Enhanced Caching

**Current Issue:** Basic caching implementation could be more sophisticated.

**Solution:** Implement tiered caching with different strategies.

```rust
pub struct CacheConfig {
    pub memory_cache_size: usize,
    pub memory_ttl: Duration,
    pub disk_cache_size: usize,
    pub disk_ttl: Duration,
}

pub struct TieredCache {
    memory: Arc<MemoryCache>,
    disk: Arc<DiskCache>,
}

impl TieredCache {
    async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, CacheError> {
        // Try memory first
        if let Some(value) = self.memory.get(key).await? {
            return Ok(Some(value));
        }

        // Try disk if not in memory
        if let Some(value) = self.disk.get(key).await? {
            // Populate memory cache
            self.memory.set(key, &value).await?;
            return Ok(Some(value));
        }

        Ok(None)
    }
}
```

### 2.2 Connection Pooling

**Current Issue:** Basic HTTP client configuration.

**Solution:** Implement sophisticated connection pooling.

```rust
pub fn create_http_client(config: &Config) -> Result<Client, ServiceError> {
    let pool_config = ConnectionPoolConfig {
        max_idle_per_host: config.http.max_idle_connections,
        max_idle_timeout: config.http.idle_timeout,
        max_lifetime: config.http.connection_lifetime,
    };

    Client::builder()
        .pool_config(pool_config)
        .timeout(config.http.timeout)
        .connect_timeout(config.http.connect_timeout)
        .tcp_keepalive(config.http.tcp_keepalive)
        .build()
        .map_err(|e| ServiceError::ConfigError(e.to_string()))
}
```

## 3. Monitoring and Observability

### 3.1 Metrics Collection

**Current Issue:** Limited visibility into service performance.

**Solution:** Implement comprehensive metrics collection.

```rust
use metrics::{counter, gauge, histogram};

pub struct Metrics {
    pub requests_total: Counter,
    pub request_duration: Histogram,
    pub cache_hits: Counter,
    pub cache_misses: Counter,
    pub rate_limit_hits: Counter,
}

impl Metrics {
    pub fn record_request(&self, duration: Duration, status: StatusCode) {
        counter!("requests_total", "status" => status.as_str()).increment(1);
        histogram!("request_duration_seconds").record(duration.as_secs_f64());
    }

    pub fn record_cache_hit(&self) {
        counter!("cache_hits_total").increment(1);
    }
}
```

### 3.2 Logging Middleware

**Current Issue:** Basic logging implementation.

**Solution:** Add structured logging middleware.

```rust
pub struct LoggingMiddleware {
    metrics: Arc<Metrics>,
}

impl<S, B> Transform<S, ServiceRequest> for LoggingMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = LoggingMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(LoggingMiddlewareService {
            service,
            metrics: self.metrics.clone(),
        }))
    }
}
```

## 4. Testing Improvements

### 4.1 Test Utilities

**Current Issue:** Limited test infrastructure.

**Solution:** Add comprehensive test utilities.

```rust
#[cfg(test)]
pub mod test_utils {
    use super::*;

    pub struct TestContext {
        pub config: Config,
        pub services: ServiceRegistry,
        pub test_client: TestClient,
    }

    impl TestContext {
        pub async fn new() -> Self {
            let config = Config::test_default();
            let services = ServiceRegistry::new(config.clone())?;
            let test_client = TestClient::new();

            Self {
                config,
                services,
                test_client,
            }
        }

        pub async fn send_request(&self, req: TestRequest) -> TestResponse {
            self.test_client.send(req).await
        }
    }

    pub fn mock_currency_response() -> CurrencyResponse {
        // Create mock response
    }
}
```

## Implementation Timeline

1. Phase 1 (Week 1-2)
   - Implement dependency injection
   - Add error handling improvements
   - Create service registry

2. Phase 2 (Week 3-4)
   - Enhance caching implementation
   - Improve connection pooling
   - Add basic metrics

3. Phase 3 (Week 5-6)
   - Implement logging middleware
   - Add comprehensive metrics
   - Enhance monitoring

4. Phase 4 (Week 7-8)
   - Improve test infrastructure
   - Add performance tests
   - Complete documentation

## Migration Strategy

1. Create feature branches for each major change
2. Implement changes with backward compatibility
3. Add tests for new features
4. Review and merge incrementally
5. Deploy changes gradually with feature flags

## Success Metrics

- Improved test coverage (target: 90%+)
- Reduced error rates (target: <0.1%)
- Improved response times (target: p95 <100ms)
- Better cache hit rates (target: >80%)
- Reduced external API calls (target: 50% reduction)

## Next Steps

1. Review and prioritize improvements
2. Create detailed technical specifications
3. Set up monitoring baseline
4. Begin incremental implementation
5. Measure and iterate on changes

---

This upgrade plan will be regularly updated as implementation progresses.
