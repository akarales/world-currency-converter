use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub exchange_rate_api_key: String,
    pub cache_settings: CacheSettings,
    pub rate_limit_settings: RateLimitSettings,
}

#[derive(Debug, Clone)]
pub struct CacheSettings {
    pub exchange_rate_ttl_minutes: i64,    // 60 minutes based on your plan
    pub country_info_ttl_minutes: i64,     // 24 hours since this rarely changes
    pub cache_cleanup_interval_minutes: i64,// Cleanup old cache entries
}

#[derive(Debug, Clone)]
pub struct RateLimitSettings {
    pub requests_per_day: usize,     // 30,000 per month ≈ 1,000 per day
    pub window_size_minutes: i64,    // Time window for rate limiting
}

impl Config {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            exchange_rate_api_key: env::var("EXCHANGE_RATE_API_KEY")
                .map_err(|_| "EXCHANGE_RATE_API_KEY not set")?,
            cache_settings: CacheSettings {
                exchange_rate_ttl_minutes: 60,          // Match API update frequency
                country_info_ttl_minutes: 24 * 60,     // 24 hours
                cache_cleanup_interval_minutes: 5,      // Clean every 5 minutes
            },
            rate_limit_settings: RateLimitSettings {
                requests_per_day: 1000,                // ~30,000 per month
                window_size_minutes: 24 * 60,          // 24 hour window
            },
        })
    }
}