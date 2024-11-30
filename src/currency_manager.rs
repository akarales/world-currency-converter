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
        if self.is_test {
            info!("Test mode: Fetching fresh currency data");
            self.fetch_currency_data().await
        } else {
            match self.load_from_file().await? {
                Some(data) if !self.should_update(&data) => {
                    debug!("Using existing currency data");
                    Ok(data)
                }
                _ => {
                    info!("Fetching fresh currency data");
                    self.fetch_currency_data().await
                }
            }
        }
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

        // Get exchange rates first
        let exchange_rates = self.get_exchange_rates().await.ok();
        
        // First, analyze currency usage patterns
        let mut currency_usage = HashMap::new();
        for country in &countries {
            if let Some(ref currencies) = country.currencies {
                for code in currencies.keys() {
                    let entry = currency_usage.entry(code.clone()).or_insert_with(Vec::new);
                    entry.push(country.name.common.clone());
                }
            }
        }

        // Process each country
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

        // Save the updated data
        if !self.is_test {
            if let Err(e) = self.save_to_file(&update_info).await {
                warn!("Failed to save currency data: {}", e);
            }
        }

        debug!("Successfully processed {} countries", update_info.data.len());
        Ok(update_info)
    }

    fn determine_primary_currency(
        &self,
        currencies: &HashMap<String, CurrencyInfo>,
        usage_patterns: &HashMap<String, Vec<String>>,
        exchange_rates: &Option<HashMap<String, f64>>,
    ) -> (String, bool) {
        // First determine if this is a multi-currency situation
        let is_multi_currency = currencies.len() > 1 || 
            currencies.keys().any(|code| {
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

    fn should_update(&self, data: &CurrencyUpdateInfo) -> bool {
        let now = Utc::now();
        now.signed_duration_since(data.last_checked).num_hours() >= 24
    }

    async fn save_to_file(&self, data: &CurrencyUpdateInfo) -> Result<(), ServiceError> {
        fs::create_dir_all(&self.config_dir)
            .map_err(|e| ServiceError::ConfigError(format!("Failed to create config directory: {}", e)))?;
        
        // Create backup directory and backup existing file if needed
        if !self.is_test {
            let backup_dir = self.config_dir.join("backups");
            if fs::create_dir_all(&backup_dir).is_ok() {
                let file_path = self.config_dir.join("country_currencies.json");
                if file_path.exists() {
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
        }
        
        let file_path = self.config_dir.join("country_currencies.json");
        let json = serde_json::to_string_pretty(&data)
            .map_err(|e| ServiceError::ConfigError(format!("Failed to serialize currency data: {}", e)))?;
        
        fs::write(&file_path, json)
            .map_err(|e| ServiceError::ConfigError(format!("Failed to write currency data: {}", e)))?;
        
        debug!("Saved currency data to {}", file_path.display());
        Ok(())
    }

    async fn load_from_file(&self) -> Result<Option<CurrencyUpdateInfo>, ServiceError> {
        let file_path = self.config_dir.join("country_currencies.json");
        
        if file_path.exists() {
            let content = fs::read_to_string(&file_path)
                .map_err(|e| ServiceError::ConfigError(format!("Failed to read currency data: {}", e)))?;
            
            let data = serde_json::from_str(&content)
                .map_err(|e| ServiceError::ConfigError(format!("Failed to parse currency data: {}", e)))?;
            
            debug!("Loaded currency data from {}", file_path.display());
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn create_test_currency_info(name: &str, symbol: &str) -> CurrencyInfo {
        CurrencyInfo {
            name: name.to_string(),
            symbol: symbol.to_string(),
        }
    }

    fn setup_test_currencies() -> HashMap<String, HashMap<String, CurrencyInfo>> {
        let mut countries = HashMap::new();
        
        // Set up Panama (multi-currency: USD and PAB)
        let mut panama = HashMap::new();
        panama.insert("USD".to_string(), create_test_currency_info("US Dollar", "$"));
        panama.insert("PAB".to_string(), create_test_currency_info("Panamanian Balboa", "B/."));
        countries.insert("Panama".to_string(), panama);

        // Set up Zimbabwe (multi-currency: ZWL and USD)
        let mut zimbabwe = HashMap::new();
        zimbabwe.insert("ZWL".to_string(), create_test_currency_info("Zimbabwean Dollar", "$"));
        zimbabwe.insert("USD".to_string(), create_test_currency_info("US Dollar", "$"));
        countries.insert("Zimbabwe".to_string(), zimbabwe);

        // Set up Switzerland (single currency but used in multiple regions)
        let mut switzerland = HashMap::new();
        switzerland.insert("CHF".to_string(), create_test_currency_info("Swiss Franc", "Fr"));
        countries.insert("Switzerland".to_string(), switzerland);

        // Set up single currency country
        let mut japan = HashMap::new();
        japan.insert("JPY".to_string(), create_test_currency_info("Japanese Yen", "¥"));
        countries.insert("Japan".to_string(), japan);

        countries
    }

    #[tokio::test]
    async fn test_currency_manager() {
        let manager = CurrencyManager::new("test_key".to_string(), true);
        let data = manager.ensure_currency_data().await.unwrap();
        
        assert!(!data.data.is_empty());
        assert!(data.data.values().any(|config| config.is_multi_currency));
        
        // Verify test directory was used
        let test_file = Path::new("config/test/country_currencies.json");
        assert!(test_file.exists());
    }

    #[tokio::test]
    async fn test_multi_currency_detection() {
        let manager = CurrencyManager::new("test_key".to_string(), true);
        let currencies = setup_test_currencies();
        let mut usage_patterns = HashMap::new();

        // Add usage patterns
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

        // Mock exchange rates
        let mut exchange_rates = HashMap::new();
        exchange_rates.insert("USD".to_string(), 1.0);
        exchange_rates.insert("EUR".to_string(), 0.85);
        exchange_rates.insert("CHF".to_string(), 0.89);
        exchange_rates.insert("JPY".to_string(), 0.0091);
        let rates = Some(exchange_rates);

        // Test Panama
        let (primary, is_multi) = manager.determine_primary_currency(
            currencies.get("Panama").unwrap(),
            &usage_patterns,
            &rates
        );
        assert_eq!(primary, "USD");
        assert!(is_multi, "Panama should be detected as multi-currency");

        // Test Zimbabwe
        let (primary, is_multi) = manager.determine_primary_currency(
            currencies.get("Zimbabwe").unwrap(),
            &usage_patterns,
            &rates
        );
        assert_eq!(primary, "USD");
        assert!(is_multi, "Zimbabwe should be detected as multi-currency");

        // Test Switzerland
        let (primary, is_multi) = manager.determine_primary_currency(
            currencies.get("Switzerland").unwrap(),
            &usage_patterns,
            &rates
        );
        assert_eq!(primary, "CHF");
        assert!(is_multi, "Switzerland should be detected as multi-currency due to shared usage");

        // Test Japan (single currency)
        let (primary, is_multi) = manager.determine_primary_currency(
            currencies.get("Japan").unwrap(),
            &usage_patterns,
            &rates
        );
        assert_eq!(primary, "JPY");
        assert!(!is_multi, "Japan should not be detected as multi-currency");
    }

    #[tokio::test]
    async fn test_currency_priority() {
        let manager = CurrencyManager::new("test_key".to_string(), true);
        let mut currencies = HashMap::new();
        
        // Add multiple currencies
        currencies.insert("EUR".to_string(), create_test_currency_info("Euro", "€"));
        currencies.insert("USD".to_string(), create_test_currency_info("US Dollar", "$"));
        currencies.insert("GBP".to_string(), create_test_currency_info("British Pound", "£"));

        let mut usage_patterns = HashMap::new();
        usage_patterns.insert("EUR".to_string(), vec!["Country1".to_string(), "Country2".to_string()]);
        usage_patterns.insert("USD".to_string(), vec!["Country1".to_string()]);
        usage_patterns.insert("GBP".to_string(), vec!["Country1".to_string()]);

        // Test USD priority
        let (primary, _) = manager.determine_primary_currency(&currencies, &usage_patterns, &None);
        assert_eq!(primary, "USD", "USD should be selected as primary when available");

        // Remove USD and test EUR priority
        let mut currencies_without_usd = currencies.clone();
        currencies_without_usd.remove("USD");
        let (primary, _) = manager.determine_primary_currency(&currencies_without_usd, &usage_patterns, &None);
        assert_eq!(primary, "EUR", "EUR should be selected as primary when USD is not available");

        // Test exchange rate priority
        let mut exchange_rates = HashMap::new();
        exchange_rates.insert("GBP".to_string(), 1.2); // Higher volume
        exchange_rates.insert("EUR".to_string(), 0.85);
        
        let mut currencies_without_usd_eur = currencies_without_usd.clone();
        currencies_without_usd_eur.remove("EUR");
        let (primary, _) = manager.determine_primary_currency(
            &currencies_without_usd_eur, 
            &usage_patterns, 
            &Some(exchange_rates)
        );
        assert_eq!(primary, "GBP", "GBP should be selected due to higher exchange rate volume");
    }

    #[tokio::test]
    async fn test_backup_creation() {
        let manager = CurrencyManager::new("test_key".to_string(), false);
        let mut currencies = HashMap::new();
        currencies.insert("USD".to_string(), create_test_currency_info("US Dollar", "$"));
        
        let mut country_currencies = HashMap::new();
        country_currencies.insert("Test Country".to_string(), CountryCurrencyConfig {
            primary_currency: "USD".to_string(),
            currencies,
            is_multi_currency: false,
        });

        let update_info = CurrencyUpdateInfo {
            last_checked: Utc::now(),
            last_modified: Utc::now(),
            data: country_currencies,
        };

        // First save
        manager.save_to_file(&update_info).await.unwrap();
        
        // Wait a second to ensure different timestamp
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        
        // Second save should create backup
        manager.save_to_file(&update_info).await.unwrap();

        // Verify backup was created
        let backup_dir = Path::new("config/backups");
        assert!(backup_dir.exists());
        let backup_count = fs::read_dir(backup_dir)
            .unwrap()
            .count();
        assert!(backup_count > 0, "Backup file should have been created");
    }

    #[tokio::test]
    async fn test_config_loading() {
        let manager = CurrencyManager::new("test_key".to_string(), true);
        let mut currencies = HashMap::new();
        currencies.insert("USD".to_string(), create_test_currency_info("US Dollar", "$"));
        
        let mut country_currencies = HashMap::new();
        country_currencies.insert("Test Country".to_string(), CountryCurrencyConfig {
            primary_currency: "USD".to_string(),
            currencies,
            is_multi_currency: false,
        });

        let test_data = CurrencyUpdateInfo {
            last_checked: Utc::now(),
            last_modified: Utc::now(),
            data: country_currencies,
        };

        // Save test data
        manager.save_to_file(&test_data).await.unwrap();

        // Load and verify
        let loaded = manager.load_from_file().await.unwrap().unwrap();
        assert_eq!(loaded.data.len(), test_data.data.len());
        
        let loaded_country = loaded.data.get("Test Country").unwrap();
        assert_eq!(loaded_country.primary_currency, "USD");
        assert!(!loaded_country.is_multi_currency);
        assert!(loaded_country.currencies.contains_key("USD"));
    }

    #[tokio::test]
    async fn test_update_check() {
        let manager = CurrencyManager::new("test_key".to_string(), true);
        let now = Utc::now();
        let day_ago = now - chrono::Duration::hours(25);
        
        let test_data = CurrencyUpdateInfo {
            last_checked: day_ago,
            last_modified: day_ago,
            data: HashMap::new(),
        };

        assert!(manager.should_update(&test_data), "Should update data older than 24 hours");

        let test_data_recent = CurrencyUpdateInfo {
            last_checked: now,
            last_modified: now,
            data: HashMap::new(),
        };

        assert!(!manager.should_update(&test_data_recent), "Should not update recent data");
    }
}