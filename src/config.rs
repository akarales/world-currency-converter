use std::env;
use log::warn;

#[derive(Debug, Clone)]
pub struct Config {
    pub exchange_rate_api_key: String,
    pub cache_settings: CacheSettings,
    pub rate_limit_settings: RateLimitSettings,
    pub api_settings: ApiSettings,
    pub currency_settings: CurrencySettings,
}

#[derive(Debug, Clone)]
pub struct CacheSettings {
    pub exchange_rate_ttl_minutes: i64,    // 60 minutes based on rate updates
    pub country_info_ttl_minutes: i64,     // 24 hours since rarely changes
    pub cache_cleanup_interval_minutes: i64,// Cleanup interval
    pub exchange_rate_max_entries: usize,  // Maximum exchange rate cache entries
    pub country_info_max_entries: usize,   // Maximum country info cache entries
}

#[derive(Debug, Clone)]
pub struct RateLimitSettings {
    pub requests_per_day: usize,     // Daily request limit
    pub window_size_minutes: i64,    // Time window for rate limiting
}

#[derive(Debug, Clone)]
pub struct ApiSettings {
    pub rest_countries_base_url: String,
    pub exchange_rate_base_url: String,
    pub connect_timeout_seconds: u64,
    pub request_timeout_seconds: u64,
    pub rate_limit_timeout_seconds: u64,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub enable_compression: bool,
    pub user_agent: String,
}

#[derive(Debug, Clone)]
pub struct CurrencySettings {
    pub config_dir: String,
    pub backup_enabled: bool,
    pub backup_retention_days: u32,
    pub update_interval_hours: u32,
    pub test_mode: bool,
}

impl Default for CurrencySettings {
    fn default() -> Self {
        Self {
            config_dir: "config".to_string(),
            backup_enabled: true,
            backup_retention_days: 30,
            update_interval_hours: 24,
            test_mode: cfg!(test),
        }
    }
}

impl Default for ApiSettings {
    fn default() -> Self {
        Self {
            rest_countries_base_url: "https://restcountries.com/v3.1".to_string(),
            exchange_rate_base_url: "https://v6.exchangerate-api.com/v6".to_string(),
            connect_timeout_seconds: 10,
            request_timeout_seconds: 30,
            rate_limit_timeout_seconds: 5,
            max_retries: 3,
            retry_delay_ms: 1000,
            enable_compression: true,
            user_agent: format!("CurrencyConverter/{}", env!("CARGO_PKG_VERSION")),
        }
    }
}

impl Config {
    pub fn new() -> Result<Self, String> {
        let exchange_rate_api_key = env::var("EXCHANGE_RATE_API_KEY")
            .map_err(|_| "EXCHANGE_RATE_API_KEY not set")?;

        // Get cache settings from environment or use defaults
        let exchange_rate_ttl = env::var("EXCHANGE_RATE_CACHE_TTL_MINUTES")
            .unwrap_or_else(|_| "60".to_string())
            .parse()
            .unwrap_or(60);

        let country_info_ttl = env::var("COUNTRY_INFO_CACHE_TTL_MINUTES")
            .unwrap_or_else(|_| "1440".to_string()) // 24 hours
            .parse()
            .unwrap_or(1440);

        // Get rate limit settings from environment or use defaults
        let requests_per_day = env::var("REQUESTS_PER_DAY")
            .unwrap_or_else(|_| "1000".to_string())
            .parse()
            .unwrap_or(1000);

        // Get currency settings from environment
        let currency_settings = CurrencySettings {
            config_dir: env::var("CURRENCY_CONFIG_DIR")
                .unwrap_or_else(|_| "config".to_string()),
            backup_enabled: env::var("CURRENCY_BACKUP_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            backup_retention_days: env::var("CURRENCY_BACKUP_RETENTION_DAYS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
            update_interval_hours: env::var("CURRENCY_UPDATE_INTERVAL_HOURS")
                .unwrap_or_else(|_| "24".to_string())
                .parse()
                .unwrap_or(24),
            test_mode: cfg!(test),
        };

        // Validate critical settings
        if exchange_rate_ttl < 1 {
            warn!("Exchange rate cache TTL is too low, using default of 60 minutes");
        }
        if country_info_ttl < 1 {
            warn!("Country info cache TTL is too low, using default of 24 hours");
        }
        if requests_per_day < 1 {
            warn!("Requests per day is too low, using default of 1000");
        }

        Ok(Self {
            exchange_rate_api_key,
            cache_settings: CacheSettings {
                exchange_rate_ttl_minutes: exchange_rate_ttl.max(1),
                country_info_ttl_minutes: country_info_ttl.max(1),
                cache_cleanup_interval_minutes: 5,
                exchange_rate_max_entries: 1000,
                country_info_max_entries: 500,
            },
            rate_limit_settings: RateLimitSettings {
                requests_per_day: requests_per_day.max(1),
                window_size_minutes: 24 * 60, // 24 hour window
            },
            api_settings: ApiSettings::default(),
            currency_settings,
        })
    }

    pub fn with_test_settings() -> Self {
        Self {
            exchange_rate_api_key: "test_key".to_string(),
            cache_settings: CacheSettings {
                exchange_rate_ttl_minutes: 5,
                country_info_ttl_minutes: 5,
                cache_cleanup_interval_minutes: 1,
                exchange_rate_max_entries: 1000,  // Match production for consistency
                country_info_max_entries: 500,    // Match production for consistency
            },
            rate_limit_settings: RateLimitSettings {
                requests_per_day: 100,
                window_size_minutes: 60,
            },
            api_settings: ApiSettings {
                rest_countries_base_url: "http://localhost:8081".to_string(),
                exchange_rate_base_url: "http://localhost:8082".to_string(),
                connect_timeout_seconds: 1,
                request_timeout_seconds: 2,
                rate_limit_timeout_seconds: 1,
                max_retries: 1,
                retry_delay_ms: 100,
                enable_compression: false,
                user_agent: "TestClient/1.0".to_string(),
            },
            currency_settings: CurrencySettings {
                config_dir: "config/test".to_string(),
                backup_enabled: false,
                backup_retention_days: 1,
                update_interval_hours: 1,
                test_mode: true,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn setup() {
        std::env::set_var("RUST_LOG", "debug");
        let _ = env_logger::try_init();
    }

    #[test]
    fn test_config_creation() {
        setup();
        env::set_var("EXCHANGE_RATE_API_KEY", "test_key");
        env::set_var("CURRENCY_BACKUP_ENABLED", "true");
        
        let config = Config::new().unwrap();
        assert_eq!(config.exchange_rate_api_key, "test_key");
        assert!(config.currency_settings.backup_enabled);
    }

    #[test]
    fn test_config_validation() {
        setup();
        env::set_var("EXCHANGE_RATE_API_KEY", "test_key");
        env::set_var("EXCHANGE_RATE_CACHE_TTL_MINUTES", "0");
        env::set_var("REQUESTS_PER_DAY", "0");

        let config = Config::new().unwrap();
        assert!(config.cache_settings.exchange_rate_ttl_minutes >= 1);
        assert!(config.rate_limit_settings.requests_per_day >= 1);
    }

    #[test]
    fn test_currency_settings() {
        setup();
        env::set_var("EXCHANGE_RATE_API_KEY", "test_key");
        env::set_var("CURRENCY_CONFIG_DIR", "custom_config");
        env::set_var("CURRENCY_BACKUP_ENABLED", "false");
    
        let config = Config::with_test_settings();
        assert_eq!(config.currency_settings.config_dir, "config/test");
        assert!(!config.currency_settings.backup_enabled);
        assert!(config.currency_settings.test_mode);
    }

    #[test]
    fn test_test_settings() {
        setup();
        let config = Config::with_test_settings();
        assert_eq!(config.currency_settings.config_dir, "config/test");
        assert!(!config.currency_settings.backup_enabled);
        assert!(config.currency_settings.test_mode);
    }
}