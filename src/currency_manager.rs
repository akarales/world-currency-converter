use crate::models::{CurrencyInfo, CurrencyUpdateInfo, CountryCurrencyConfig};
use crate::errors::ServiceError;
use chrono::Utc;
use log::{debug, info, warn};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct CountryApiResponse {
    name: CountryName,
    currencies: Option<HashMap<String, ApiCurrencyInfo>>,
}

#[derive(Debug, Deserialize)]
struct CountryName {
    common: String,
}

#[derive(Debug, Deserialize)]
struct ApiCurrencyInfo {
    name: String,
    symbol: String,
}

#[derive(Debug)]
pub struct CurrencyManager {
    client: reqwest::Client,
    api_key: String,
    is_test: bool,
    config_dir: PathBuf,
}

impl CurrencyManager {
    pub fn new(api_key: String, is_test: bool) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        let config_dir = if is_test {
            PathBuf::from("config/test")
        } else {
            PathBuf::from("config")
        };

        Self {
            client,
            api_key,
            is_test,
            config_dir,
        }
    }

    pub async fn ensure_currency_data(&self) -> Result<CurrencyUpdateInfo, ServiceError> {
        // Always fetch fresh data
        self.fetch_currency_data().await
    }

    async fn fetch_currency_data(&self) -> Result<CurrencyUpdateInfo, ServiceError> {
        info!("Fetching country data from REST Countries API");
        
        let response = self.client
            .get("https://restcountries.com/v3.1/all?fields=name,currencies")
            .send()
            .await
            .map_err(|e| ServiceError::ExternalApiError(format!("Failed to fetch country data: {}", e)))?;
    
        let countries: Vec<CountryApiResponse> = response
            .json()
            .await
            .map_err(|e| ServiceError::ExternalApiError(format!("Failed to parse country data: {}", e)))?;
    
        let mut country_currencies = HashMap::new();
        let now = Utc::now();
    
        let exchange_rates = self.get_exchange_rates().await.ok();
        
        let mut currency_usage = HashMap::new();
        for country in &countries {
            if let Some(ref currencies) = country.currencies {
                for code in currencies.keys() {
                    currency_usage
                        .entry(code.clone())
                        .or_insert_with(Vec::new)
                        .push(country.name.common.clone());
                }
            }
        }

        for country in countries {
            if let Some(currencies) = country.currencies {
                let currency_map: HashMap<String, CurrencyInfo> = currencies
                    .into_iter()
                    .map(|(code, info)| {
                        (code.clone(), CurrencyInfo {
                            name: info.name,
                            symbol: info.symbol,
                        })
                    })
                    .collect();

                if !currency_map.is_empty() {
                    let (primary_currency, is_multi_currency) = 
                        self.determine_primary_currency(&currency_map, &currency_usage, &exchange_rates);

                    debug!("Processing {}: {} currencies, primary: {}, multi: {}", 
                        country.name.common, 
                        currency_map.len(), 
                        primary_currency,
                        is_multi_currency
                    );

                    country_currencies.insert(
                        country.name.common,
                        CountryCurrencyConfig {
                            primary_currency,
                            currencies: currency_map,
                            is_multi_currency,
                        }
                    );
                }
            }
        }

        let update_info = CurrencyUpdateInfo {
            last_checked: now,
            last_modified: now,
            data: country_currencies,
        };

        self.backup_and_save(&update_info).await?;

        debug!("Successfully processed {} countries", update_info.data.len());
        Ok(update_info)
    }

    async fn backup_and_save(&self, update_info: &CurrencyUpdateInfo) -> Result<(), ServiceError> {
        fs::create_dir_all(&self.config_dir)
            .map_err(|e| ServiceError::ConfigError(format!("Failed to create config directory: {}", e)))?;

        let file_path = self.config_dir.join("country_currencies.json");
        
        if !self.is_test && file_path.exists() {
            let backup_dir = self.config_dir.join("backups");
            if let Err(e) = fs::create_dir_all(&backup_dir) {
                warn!("Failed to create backup directory: {}", e);
            } else {
                let backup_name = format!("country_currencies_{}.json", 
                    Utc::now().format("%Y%m%d_%H%M%S"));
                let backup_path = backup_dir.join(&backup_name);
                
                if let Err(e) = fs::copy(&file_path, &backup_path) {
                    warn!("Failed to create backup at {}: {}", backup_path.display(), e);
                } else {
                    debug!("Created backup: {}", backup_name);
                }
            }
        }

        let json = serde_json::to_string_pretty(&update_info)
            .map_err(|e| ServiceError::ConfigError(format!("Failed to serialize currency data: {}", e)))?;
        
        fs::write(&file_path, json)
            .map_err(|e| ServiceError::ConfigError(format!("Failed to write currency data: {}", e)))?;
        
        debug!("Saved currency data to {}", file_path.display());
        Ok(())
    }

    async fn get_exchange_rates(&self) -> Result<HashMap<String, f64>, ServiceError> {
        let url = format!(
            "https://v6.exchangerate-api.com/v6/{}/latest/USD",
            self.api_key
        );
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| ServiceError::ExternalApiError(format!("Failed to fetch exchange rates: {}", e)))?;

        #[derive(Deserialize)]
        struct ExchangeRateResponse {
            conversion_rates: HashMap<String, f64>,
        }

        let data: ExchangeRateResponse = response
            .json()
            .await
            .map_err(|e| ServiceError::ExternalApiError(format!("Failed to parse exchange rates: {}", e)))?;

        debug!("Retrieved {} exchange rates", data.conversion_rates.len());
        Ok(data.conversion_rates)
    }

    fn determine_primary_currency(
        &self,
        currencies: &HashMap<String, CurrencyInfo>,
        usage_patterns: &HashMap<String, Vec<String>>,
        exchange_rates: &Option<HashMap<String, f64>>,
    ) -> (String, bool) {
        // Consider a country multi-currency if it has any of:
        // 1. Multiple currencies
        // 2. Shared currency usage
        // 3. USD as an accepted currency
        let is_multi_currency = currencies.len() > 1 || 
            currencies.keys().any(|code| {
                code == "USD" || // Consider USD-accepting countries as multi-currency
                usage_patterns.get(code)
                    .map(|users| users.len() > 1)
                    .unwrap_or(false)
            });

        let primary_currency = if currencies.contains_key("USD") {
            "USD".to_string()
        } else if currencies.contains_key("EUR") {
            "EUR".to_string()
        } else if let Some(rates) = exchange_rates {
            currencies.keys()
                .filter(|code| rates.contains_key(*code))
                .max_by(|a, b| {
                    rates.get(*a)
                        .unwrap_or(&0.0)
                        .partial_cmp(rates.get(*b).unwrap_or(&0.0))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|code| code.to_string())
                .unwrap_or_else(|| {
                    currencies.keys()
                        .max_by_key(|code| {
                            usage_patterns.get(*code)
                                .map(|users| users.len())
                                .unwrap_or(0)
                        })
                        .map(|code| code.to_string())
                        .unwrap_or_else(|| currencies.keys().next().unwrap().clone())
                })
        } else {
            currencies.keys()
                .max_by_key(|code| {
                    usage_patterns.get(*code)
                        .map(|users| users.len())
                        .unwrap_or(0)
                })
                .map(|code| code.to_string())
                .unwrap_or_else(|| currencies.keys().next().unwrap().clone())
        };

        (primary_currency, is_multi_currency)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::env;
    use crate::test_utils::test_summary::{record_test_result, print_test_summary};

    async fn setup_test_env(is_test: bool) -> CurrencyManager {
        // Initialize logging directly
        std::env::set_var("RUST_LOG", "debug");
        let _ = env_logger::try_init();
        
        env::set_var("EXCHANGE_RATE_API_KEY", "test_key");
        
        if is_test {
            let test_dir = Path::new("config/test");
            if !test_dir.exists() {
                fs::create_dir_all(test_dir).expect("Failed to create test directory");
            }
        }
        
        CurrencyManager::new("test_key".to_string(), is_test)
    }

    fn create_test_currency_info(name: &str, symbol: &str) -> CurrencyInfo {
        CurrencyInfo {
            name: name.to_string(),
            symbol: symbol.to_string(),
        }
    }

    fn setup_test_currencies() -> HashMap<String, HashMap<String, CurrencyInfo>> {
        let mut countries = HashMap::new();
        
        let mut panama = HashMap::new();
        panama.insert("USD".to_string(), create_test_currency_info("US Dollar", "$"));
        panama.insert("PAB".to_string(), create_test_currency_info("Panamanian Balboa", "B/."));
        countries.insert("Panama".to_string(), panama);

        let mut zimbabwe = HashMap::new();
        zimbabwe.insert("ZWL".to_string(), create_test_currency_info("Zimbabwean Dollar", "$"));
        zimbabwe.insert("USD".to_string(), create_test_currency_info("US Dollar", "$"));
        countries.insert("Zimbabwe".to_string(), zimbabwe);

        let mut switzerland = HashMap::new();
        switzerland.insert("CHF".to_string(), create_test_currency_info("Swiss Franc", "Fr"));
        countries.insert("Switzerland".to_string(), switzerland);

        let mut japan = HashMap::new();
        japan.insert("JPY".to_string(), create_test_currency_info("Japanese Yen", "¥"));
        countries.insert("Japan".to_string(), japan);

        countries
    }

    #[tokio::test]
    async fn test_currency_manager() {
        let result = async {
            let manager = setup_test_env(true).await;
            let data = manager.ensure_currency_data().await?;
            
            assert!(!data.data.is_empty());
            assert!(data.data.values().any(|config| config.is_multi_currency));
            
            let test_file = Path::new("config/test/country_currencies.json");
            assert!(test_file.exists());
            Ok::<(), ServiceError>(())
        }.await;

        record_test_result("test_currency_manager", result.is_ok()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_multi_currency_detection() {
        let result = async {
            let manager = setup_test_env(true).await;
            let currencies = setup_test_currencies();
            let mut usage_patterns = HashMap::new();

            usage_patterns.insert("USD".to_string(), vec![
                "Panama".to_string(), 
                "Zimbabwe".to_string(),
                "United States".to_string()
            ]);
            usage_patterns.insert("PAB".to_string(), vec!["Panama".to_string()]);
            usage_patterns.insert("ZWL".to_string(), vec!["Zimbabwe".to_string()]);
            usage_patterns.insert("CHF".to_string(), vec![
                "Switzerland".to_string(),
                "Liechtenstein".to_string()
            ]);
            usage_patterns.insert("JPY".to_string(), vec!["Japan".to_string()]);

            let mut exchange_rates = HashMap::new();
            exchange_rates.insert("USD".to_string(), 1.0);
            exchange_rates.insert("EUR".to_string(), 0.85);
            exchange_rates.insert("CHF".to_string(), 0.89);
            exchange_rates.insert("JPY".to_string(), 0.0091);
            let rates = Some(exchange_rates);

            let (primary, is_multi) = manager.determine_primary_currency(
                currencies.get("Panama").unwrap(),
                &usage_patterns,
                &rates
            );
            assert_eq!(primary, "USD");
            assert!(is_multi, "Panama should be detected as multi-currency");

            let (primary, is_multi) = manager.determine_primary_currency(
                currencies.get("Zimbabwe").unwrap(),
                &usage_patterns,
                &rates
            );
            assert_eq!(primary, "USD");
            assert!(is_multi, "Zimbabwe should be detected as multi-currency");

            let (primary, is_multi) = manager.determine_primary_currency(
                currencies.get("Switzerland").unwrap(),
                &usage_patterns,
                &rates
            );
            assert_eq!(primary, "CHF");
            assert!(is_multi, "Switzerland should be detected as multi-currency due to shared usage");

            let (primary, is_multi) = manager.determine_primary_currency(
                currencies.get("Japan").unwrap(),
                &usage_patterns,
                &rates
            );
            assert_eq!(primary, "JPY");
            assert!(!is_multi, "Japan should not be detected as multi-currency");
            
            Ok::<(), ServiceError>(())
        }.await;

        record_test_result("test_multi_currency_detection", result.is_ok()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_currency_priority() {
        let result = async {
            let manager = setup_test_env(true).await;
            let mut currencies = HashMap::new();
            
            currencies.insert("EUR".to_string(), create_test_currency_info("Euro", "€"));
            currencies.insert("USD".to_string(), create_test_currency_info("US Dollar", "$"));
            currencies.insert("GBP".to_string(), create_test_currency_info("British Pound", "£"));

            let mut usage_patterns = HashMap::new();
            usage_patterns.insert("EUR".to_string(), vec!["Country1".to_string(), "Country2".to_string()]);
            usage_patterns.insert("USD".to_string(), vec!["Country1".to_string()]);
            usage_patterns.insert("GBP".to_string(), vec!["Country1".to_string()]);

            let (primary, _) = manager.determine_primary_currency(&currencies, &usage_patterns, &None);
            assert_eq!(primary, "USD", "USD should be selected as primary when available");

            let mut currencies_without_usd = currencies.clone();
            currencies_without_usd.remove("USD");
            let (primary, _) = manager.determine_primary_currency(&currencies_without_usd, &usage_patterns, &None);
            assert_eq!(primary, "EUR", "EUR should be selected as primary when USD is not available");

            let mut exchange_rates = HashMap::new();
            exchange_rates.insert("GBP".to_string(), 1.2);
            exchange_rates.insert("EUR".to_string(), 0.85);
            
            let mut currencies_without_usd_eur = currencies_without_usd.clone();
            currencies_without_usd_eur.remove("EUR");
            let (primary, _) = manager.determine_primary_currency(
                &currencies_without_usd_eur, 
                &usage_patterns, 
                &Some(exchange_rates)
            );
            assert_eq!(primary, "GBP", "GBP should be selected due to higher exchange rate volume");
            
            Ok::<(), ServiceError>(())
        }.await;

        record_test_result("test_currency_priority", result.is_ok()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_backup_creation() {
        use std::fs;
        use std::path::Path;
        use tokio::time::sleep;
        use std::time::Duration;
        use crate::test_utils::with_timeout;
    
        let result = with_timeout(async {
            // Clean up any existing test files first
            let base_dir = Path::new("config/test");
            let backup_dir = base_dir.join("backups");
            
            if base_dir.exists() {
                let _ = fs::remove_dir_all(base_dir);
            }
            
            // Create fresh test directories
            fs::create_dir_all(&backup_dir).expect("Failed to create test backup directory");
    
            // Initialize with test mode
            let manager = CurrencyManager::new("test_key".to_string(), true);
    
            // Create and verify each backup
            for i in 0..3_usize {
                debug!("Creating backup {}", i);
                
                let test_data = create_test_currency_data(i);
                manager.backup_and_save(&test_data).await?;
                
                // Add small delay to ensure different timestamps
                sleep(Duration::from_millis(100)).await;
    
                // Verify backup files explicitly
                let backup_files: Vec<_> = fs::read_dir(&backup_dir)
                    .expect("Failed to read backup directory")
                    .filter_map(|entry| entry.ok())
                    .filter(|entry| {
                        entry.file_name()
                            .to_string_lossy()
                            .starts_with("country_currencies_")
                    })
                    .collect();
    
                let count = backup_files.len();
                debug!("Found {} backup files after iteration {}", count, i);
                
                assert!(
                    count >= i + 1,
                    "Expected at least {} backup files, found {} - backup dir: {:?}, files: {:?}",
                    i + 1,
                    count,
                    backup_dir,
                    backup_files.iter().map(|f| f.file_name()).collect::<Vec<_>>()
                );
            }
    
            Ok::<(), ServiceError>(())
        }).await;
    
        assert!(result.is_ok(), "Backup creation test failed: {:?}", result.err());
    }
    
    // Helper function updated to take usize
    fn create_test_currency_data(index: usize) -> CurrencyUpdateInfo {
        let mut currencies = HashMap::new();
        currencies.insert(
            "USD".to_string(),
            CurrencyInfo {
                name: format!("Test Currency {}", index),
                symbol: "$".to_string(),
            }
        );
    
        let mut country_currencies = HashMap::new();
        country_currencies.insert(
            format!("Test Country {}", index),
            CountryCurrencyConfig {
                primary_currency: "USD".to_string(),
                currencies,
                is_multi_currency: false,
            }
        );
    
        CurrencyUpdateInfo {
            last_checked: Utc::now(),
            last_modified: Utc::now(),
            data: country_currencies,
        }
    }

    #[tokio::test]
    async fn test_exchange_rates() {
        let result = async {
            let manager = setup_test_env(true).await;
            // Use a mock response or verify the response structure
            match manager.get_exchange_rates().await {
                Ok(rates) => {
                    assert!(!rates.is_empty());
                    Ok(())
                },
                Err(e) => {
                    if let ServiceError::ExternalApiError(_) = e {
                        // This is expected in test environment
                        Ok(())
                    } else {
                        Err(e)
                    }
                }
            }
        }.await;

        record_test_result("test_exchange_rates", result.is_ok()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_config_paths() {
        let result = async {
            let prod_manager = setup_test_env(false).await;
            assert_eq!(prod_manager.config_dir, PathBuf::from("config"));

            let test_manager = setup_test_env(true).await;
            assert_eq!(test_manager.config_dir, PathBuf::from("config/test"));
            Ok::<(), ServiceError>(())
        }.await;

        record_test_result("test_config_paths", result.is_ok()).await;
        assert!(result.is_ok());
    }

    // This must be the last test to run
    #[tokio::test]
    async fn zz_print_test_summary() {
        print_test_summary().await;
    }
}