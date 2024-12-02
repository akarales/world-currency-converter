pub async fn health_check() -> actix_web::Result<&'static str> {
    Ok("OK")
}

pub mod registry;
pub mod handlers;
pub mod handlers_v1;
pub mod models;
pub mod cache;
pub mod config;
pub mod monitor;
pub mod rate_limit;
pub mod currency_service;
pub mod errors;
pub mod clients;
pub mod data;
pub mod update_service;
pub mod currency_manager;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils {
    use tokio::time::{timeout, Duration};
    use std::future::Future;
    use std::sync::Once;
    use std::env;

    static INIT: Once = Once::new();
    pub static TEST_TIMEOUT: Duration = Duration::from_secs(5);

    pub async fn with_timeout<F, T>(f: F) -> T 
    where
        F: Future<Output = T>,
    {
        match timeout(TEST_TIMEOUT, f).await {
            Ok(result) => result,
            Err(_) => panic!("Test timed out after {} seconds", TEST_TIMEOUT.as_secs()),
        }
    }

    pub async fn init_test_env() {
        INIT.call_once(|| {
            env::set_var("RUST_LOG", "debug");
            if env::var("RUST_TEST").is_err() {
                env::set_var("RUST_TEST", "1");
            }
            if env::var("EXCHANGE_RATE_API_KEY").is_err() {
                env::set_var("EXCHANGE_RATE_API_KEY", "test_key");
            }
            if env::var("CURRENCY_BACKUP_ENABLED").is_err() {
                env::set_var("CURRENCY_BACKUP_ENABLED", "true");
            }
            if env::var("CURRENCY_CONFIG_DIR").is_err() {
                env::set_var("CURRENCY_CONFIG_DIR", "config/test");
            }
            env_logger::builder()
                .is_test(true)
                .try_init()
                .ok();
        });
        ensure_test_dirs();
    }

    pub fn ensure_test_dirs() {
        use std::fs;
        let test_dirs = ["config/test", "config/test/backups"];
        for dir in test_dirs {
            fs::create_dir_all(dir).unwrap_or_else(|e| {
                panic!("Failed to create test directory {}: {}", dir, e);
            });
        }
    }

    pub mod setup {
        use std::fs;
        use std::path::Path;
        use std::error::Error;

        pub async fn cleanup_test_env() -> Result<(), Box<dyn Error>> {
            let test_path = Path::new("config/test");
            if test_path.exists() {
                fs::remove_dir_all(test_path)?;
            }
            Ok(())
        }
    }

    pub mod mocks {
        use crate::errors::ServiceError;
        use crate::models::{CountryInfo, ExchangeRateResponse, CountryName, CurrencyInfo};
        use async_trait::async_trait;
        use std::collections::HashMap;
        use log::debug;

        #[derive(Default)]
        pub struct MockHttpClient;

        #[async_trait]
        impl crate::clients::CountryClient for MockHttpClient {
            async fn get_country_info(&self, country_name: &str) -> Result<CountryInfo, ServiceError> {
                if country_name == "Narnia" {
                    return Err(ServiceError::CountryNotFound(country_name.to_string()));
                }

                let currencies = match country_name {
                    "Zimbabwe" => {
                        let mut map = HashMap::new();
                        map.insert(
                            "USD".to_string(),
                            CurrencyInfo {
                                name: "US Dollar".to_string(),
                                symbol: "$".to_string(),
                            }
                        );
                        map.insert(
                            "ZWL".to_string(),
                            CurrencyInfo {
                                name: "Zimbabwean dollar".to_string(),
                                symbol: "$".to_string(),
                            }
                        );
                        map
                    },
                    "France" => {
                        let mut map = HashMap::new();
                        map.insert(
                            "EUR".to_string(),
                            CurrencyInfo {
                                name: "Euro".to_string(),
                                symbol: "€".to_string(),
                            }
                        );
                        map
                    },
                    _ => {
                        let mut map = HashMap::new();
                        map.insert(
                            "USD".to_string(),
                            CurrencyInfo {
                                name: "US Dollar".to_string(),
                                symbol: "$".to_string(),
                            }
                        );
                        map
                    }
                };

                debug!("Mock: Returning country info for {} with currencies: {:?}", country_name, currencies);

                Ok(CountryInfo {
                    name: CountryName {
                        common: country_name.to_string(),
                        official: format!("Official {}", country_name),
                        native_name: None,
                    },
                    currencies: Some(currencies),
                })
            }
        }

        #[async_trait]
        impl crate::clients::ExchangeRateClient for MockHttpClient {
            async fn get_exchange_rate(&self, from: &str) -> Result<ExchangeRateResponse, ServiceError> {
                debug!("Mock: Getting exchange rate for {}", from);
                
                let mut rates = HashMap::new();
                rates.insert("USD".to_string(), 1.0);
                rates.insert("EUR".to_string(), 0.85);
                rates.insert("GBP".to_string(), 0.73);
                rates.insert("ZWL".to_string(), 322.0);
                
                Ok(ExchangeRateResponse {
                    result: "success".to_string(),
                    conversion_rates: rates,
                    time_last_update_utc: Some("2024-12-01".to_string()),
                })
            }
        }
    }

    pub mod test_summary {
        use std::collections::HashMap;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use tokio::sync::Mutex;
        use lazy_static::lazy_static;
        use std::sync::Once;
        use colored::Colorize;

        lazy_static! {
            static ref TEST_RESULTS: Mutex<HashMap<String, bool>> = Mutex::new(HashMap::new());
            static ref TOTAL_TESTS: AtomicUsize = AtomicUsize::new(0);
            static ref PASSED_TESTS: AtomicUsize = AtomicUsize::new(0);
            static ref INIT: Once = Once::new();
        }

        pub async fn record_test_result(name: &str, passed: bool) {
            let mut results = TEST_RESULTS.lock().await;
            results.insert(name.to_string(), passed);
            TOTAL_TESTS.fetch_add(1, Ordering::SeqCst);
            if passed {
                PASSED_TESTS.fetch_add(1, Ordering::SeqCst);
            }
        }

        pub async fn print_test_summary() {
            let results = TEST_RESULTS.lock().await;
            let total = TOTAL_TESTS.load(Ordering::SeqCst);
            let passed = PASSED_TESTS.load(Ordering::SeqCst);
            let failed = total - passed;

            println!("\n{}", "=== Test Summary ===".bold());
            println!("{}: {}", "Total Tests Run".bold(), total);
            println!("{}: {}", "Tests Passed".bold().green(), passed);
            println!("{}: {}", "Tests Failed".bold().red(), failed);
            println!("\n{}", "Detailed Results:".bold());

            let mut categories: HashMap<&str, Vec<(&str, bool)>> = HashMap::new();
            
            for (test_name, passed) in results.iter() {
                let category = if test_name.contains("currency_manager") {
                    "Currency Manager"
                } else if test_name.contains("multi_currency") {
                    "Multi-Currency Support"
                } else if test_name.contains("backup") {
                    "Backup Operations"
                } else if test_name.contains("config") {
                    "Configuration"
                } else if test_name.contains("exchange") {
                    "Exchange Rates"
                } else {
                    "Other Tests"
                };

                categories.entry(category).or_default().push((test_name, *passed));
            }

            for (category, tests) in categories.iter() {
                println!("\n{}:", category.bold().blue());
                for (test_name, passed) in tests {
                    let status = if *passed {
                        "✓".green().bold()
                    } else {
                        "✗".red().bold()
                    };
                    println!("  {} {}", status, test_name);
                }
            }

            println!("\n{}", "=== End Test Summary ===".bold());
        }
    }
}

pub use errors::{ServiceError, ErrorResponse};

/// Formats a country name for consistent usage.
pub fn format_country_name(name: &str) -> String {
    name.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let mut word = first.to_uppercase().collect::<String>();
                    word.extend(chars.map(|c| c.to_lowercase().next().unwrap_or(c)));
                    word
                }
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

/// Rounds a number to two decimal places for currency display
pub fn round_to_cents(amount: f64) -> f64 {
    (amount * 100.0).round() / 100.0
}

// Re-export test utilities when the test-utils feature is enabled
#[cfg(feature = "test-utils")]
pub use test_utils::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CurrencyInfo, CountryInfo, CountryName};
    use std::collections::HashMap;
    use crate::test_utils::test_summary::{record_test_result, print_test_summary};
    use crate::test_utils::with_timeout;


    #[tokio::test]
    async fn test_format_country_name() {
        let result = with_timeout(async {
            let test_cases = vec![
                ("united states", "United States"),
                ("JAPAN", "Japan"),
                ("new   zealand", "New Zealand"),
                ("great  britain", "Great Britain"),
                ("   france   ", "France"),
            ];

            for (input, expected) in test_cases {
                assert_eq!(format_country_name(input), expected);
            }
            Ok::<(), Box<dyn std::error::Error>>(())
        }).await;

        record_test_result("test_format_country_name", result.is_ok()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_round_to_cents() {
        let result = with_timeout(async {
            let test_cases = vec![
                (10.456, 10.46),
                (10.454, 10.45),
                (0.0, 0.0),
                (99.999, 100.0),
                (-10.456, -10.46),
            ];

            for (input, expected) in test_cases {
                assert_eq!(round_to_cents(input), expected);
            }
            Ok::<(), Box<dyn std::error::Error>>(())
        }).await;

        record_test_result("test_round_to_cents", result.is_ok()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_country_info_creation() {
        let result = with_timeout(async {
            let mut currencies = HashMap::new();
            currencies.insert(
                "USD".to_string(),
                CurrencyInfo {
                    name: "United States Dollar".to_string(),
                    symbol: "$".to_string(),
                },
            );

            let country = CountryInfo {
                name: CountryName {
                    common: "United States".to_string(),
                    official: "United States of America".to_string(),
                    native_name: None,
                },
                currencies: Some(currencies),
            };

            assert_eq!(country.name.common, "United States");
            assert_eq!(country.currencies.as_ref().expect("Currencies missing").len(), 1);
                    
            if let Some(usd) = country.currencies.as_ref().expect("Currencies missing").get("USD") {
                assert_eq!(usd.symbol, "$");
                assert_eq!(usd.name, "United States Dollar");
            } else {
                panic!("USD currency not found");
            }
            Ok::<(), Box<dyn std::error::Error>>(())
        }).await;

        record_test_result("test_country_info_creation", result.is_ok()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_currency_formatting() {
        let result = with_timeout(async {
            assert_eq!(format!("{:.2}", round_to_cents(10.456)), "10.46");
            assert_eq!(format!("{:.2}", round_to_cents(10.454)), "10.45");
            Ok::<(), Box<dyn std::error::Error>>(())
        }).await;

        record_test_result("test_currency_formatting", result.is_ok()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_currency_math() {
        let result = with_timeout(async {
            let test_cases = vec![
                // Basic test case
                (100.0, 0.85, 85.0, "Basic conversion"),
                // Floating point precision test
                (33.33, 1.2, 40.0, "Floating point precision"),
                // Near zero test
                (0.01, 0.85, 0.01, "Near zero handling"),
                // Large number test
                (999999.99, 0.85, 849999.99, "Large number handling"),
            ];

            for (amount, rate, expected, test_name) in test_cases {
                let converted = round_to_cents(amount * rate);
                assert_eq!(
                    converted, 
                    expected,
                    "{} failed: {} * {} = {}, expected {}",
                    test_name, amount, rate, converted, expected
                );
            }

            Ok::<(), Box<dyn std::error::Error>>(())
        }).await;

        record_test_result("test_currency_math", result.is_ok()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_invalid_country() {
        let result = with_timeout(async {
            let test_cases = vec![
                "",
                "  ",
                "123",
                "!@#$",
                "Invalid Country Name",
            ];

            for invalid_name in test_cases {
                let formatted = format_country_name(invalid_name);
                assert!(
                    !formatted.chars().any(|c| c.is_ascii_punctuation()), 
                    "Invalid characters in formatted name: {}", 
                    formatted
                );
            }

            Ok::<(), Box<dyn std::error::Error>>(())
        }).await;

        record_test_result("test_invalid_country", result.is_ok()).await;
        assert!(result.is_ok());
    }

    // This must be the last test to run
    #[tokio::test]
    async fn zz_print_test_summary() {
        print_test_summary().await;
    }
}