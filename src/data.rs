use std::sync::Arc;
use tokio::sync::{RwLock, OnceCell};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use log::{debug, info, error};
use std::time::Duration;
use crate::update_service::UpdateService;
use crate::errors::ServiceError;
use crate::models::CurrencyInfo;
use crate::currency_manager::CurrencyManager;

const DATA_LOAD_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountryData {
    pub name: String,
    pub code: String,
    pub currencies: HashMap<String, CurrencyInfo>,
    pub primary_currency: String,
    pub is_multi_currency: bool,
}

#[derive(Debug)]
pub struct GlobalData {
    pub client: reqwest::Client,
    countries: Arc<RwLock<HashMap<String, CountryData>>>,
    currencies: Arc<RwLock<HashMap<String, CurrencyInfo>>>,
    initialized: Arc<RwLock<bool>>,
    update_service: Arc<UpdateService>,
    currency_manager: Arc<CurrencyManager>,
}

impl GlobalData {
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(DATA_LOAD_TIMEOUT)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        let is_test = cfg!(test);
        
        Self {
            client: client.clone(),
            countries: Arc::new(RwLock::new(HashMap::new())),
            currencies: Arc::new(RwLock::new(HashMap::new())),
            initialized: Arc::new(RwLock::new(false)),
            update_service: Arc::new(UpdateService::new(api_key.clone())),
            currency_manager: Arc::new(CurrencyManager::new(api_key, is_test)),
        }
    }

    pub async fn ensure_initialized(&self) {
        let already_initialized = {
            let initialized = self.initialized.read().await;
            *initialized
        };

        if !already_initialized {
            let mut initialized = self.initialized.write().await;
            if !*initialized {
                self.update_service.start().await;
                
                let load_result = if cfg!(test) {
                    tokio::time::timeout(DATA_LOAD_TIMEOUT, self.load_data()).await
                        .unwrap_or_else(|_| {
                            error!("Data loading timed out");
                            Err(ServiceError::InitializationError(
                                "Data loading timed out".to_string()
                            ))
                        })
                } else {
                    self.load_data().await
                };

                if let Err(e) = load_result {
                    error!("Failed to initialize global data: {}", e);
                }
                
                *initialized = true;
            }
        }
    }

    pub async fn is_multi_currency(&self, country_name: &str) -> bool {
        let countries = self.countries.read().await;
        if let Some(country) = countries.get(country_name) {
            country.is_multi_currency
        } else {
            false
        }
    }

    pub async fn get_primary_currency(&self, country_name: &str) -> Option<String> {
        let countries = self.countries.read().await;
        countries.get(country_name)
            .map(|c| c.primary_currency.clone())
    }

    pub async fn get_country(&self, name: &str) -> Option<CountryData> {
        self.ensure_initialized().await;
        self.countries.read().await.get(name).cloned()
    }

    pub async fn get_currency(&self, code: &str) -> Option<CurrencyInfo> {
        self.ensure_initialized().await;
        self.currencies.read().await.get(code).cloned()
    }

    pub async fn get_available_currencies(&self, country_name: &str) -> Option<HashMap<String, CurrencyInfo>> {
        self.ensure_initialized().await;
        self.countries.read().await
            .get(country_name)
            .map(|c| c.currencies.clone())
    }

    async fn load_data(&self) -> Result<(), ServiceError> {
        info!("Starting to load country and currency data...");
        
        let currency_data = if cfg!(test) {
            tokio::time::timeout(
                Duration::from_secs(5),
                self.currency_manager.ensure_currency_data()
            ).await.unwrap_or_else(|_| {
                Err(ServiceError::InitializationError(
                    "Currency data fetch timed out".to_string()
                ))
            })?
        } else {
            self.currency_manager.ensure_currency_data().await?
        };

        {
            let mut countries_map = self.countries.write().await;
            let mut currencies_map = self.currencies.write().await;

            // Clear existing data
            countries_map.clear();
            currencies_map.clear();

            for (country_name, config) in currency_data.data {
                if cfg!(test) {
                    // Yield periodically during testing to prevent timeouts
                    if countries_map.len() % 50 == 0 {
                        tokio::task::yield_now().await;
                    }
                }

                let country_data = CountryData {
                    name: country_name.clone(),
                    code: country_name.clone(),
                    currencies: config.currencies.clone(),
                    primary_currency: config.primary_currency.clone(),
                    is_multi_currency: config.is_multi_currency,
                };

                countries_map.insert(country_name, country_data);

                for (code, info) in config.currencies {
                    currencies_map.insert(code, info);
                }
            }

            debug!(
                "Loaded {} countries and {} currencies", 
                countries_map.len(),
                currencies_map.len()
            );
        }

        Ok(())
    }

    #[cfg(test)]
    pub async fn clear_test_data(&self) {
        let mut countries = self.countries.write().await;
        let mut currencies = self.currencies.write().await;
        let mut initialized = self.initialized.write().await;
        
        countries.clear();
        currencies.clear();
        *initialized = false;
    }
}

pub static GLOBAL_DATA: OnceCell<GlobalData> = OnceCell::const_new();

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use serde_json::json;
    use crate::test_utils::{init_test_env, with_timeout};

    async fn setup_test_environment() -> GlobalData {
        let test_dir = Path::new("config/test");
        if !test_dir.exists() {
            fs::create_dir_all(test_dir).expect("Failed to create test directory");
        }
        GlobalData::new("test_key".to_string())
    }

    async fn cleanup_test_environment(data: &GlobalData) {
        data.clear_test_data().await;
        let _ = fs::remove_file("config/test/country_currencies.json");
    }

    #[tokio::test]
    async fn test_initialization() {
        let data = setup_test_environment().await;
        assert!(!*data.initialized.read().await);
        
        tokio::time::timeout(
            Duration::from_secs(5),
            data.ensure_initialized()
        ).await.expect("Initialization timed out");
        
        assert!(*data.initialized.read().await);
        cleanup_test_environment(&data).await;
    }

    #[tokio::test]
    async fn test_multi_currency_detection() {
        let data = setup_test_environment().await;
        
        {
            let mut countries = data.countries.write().await;
            let mut currencies = HashMap::new();
            currencies.insert("USD".to_string(), CurrencyInfo {
                name: "US Dollar".to_string(),
                symbol: "$".to_string(),
            });
            currencies.insert("EUR".to_string(), CurrencyInfo {
                name: "Euro".to_string(),
                symbol: "€".to_string(),
            });

            countries.insert(
                "Test Country".to_string(),
                CountryData {
                    name: "Test Country".to_string(),
                    code: "TST".to_string(),
                    currencies,
                    primary_currency: "USD".to_string(),
                    is_multi_currency: true,
                },
            );
        }

        assert!(data.is_multi_currency("Test Country").await);
        assert_eq!(data.get_primary_currency("Test Country").await, Some("USD".to_string()));
        cleanup_test_environment(&data).await;
    }

    #[tokio::test]
    async fn test_currency_loading() {
        let data = setup_test_environment().await;
        
        with_timeout(async {
            init_test_env();

            let test_file = Path::new("config/test/country_currencies.json");
            let test_data = json!({
                "last_checked": "2024-12-01T00:00:00Z",
                "last_modified": "2024-12-01T00:00:00Z",
                "data": {
                    "Zimbabwe": {
                        "primary_currency": "USD",
                        "currencies": {
                            "USD": {
                                "name": "US Dollar",
                                "symbol": "$"
                            },
                            "ZWL": {
                                "name": "Zimbabwean Dollar",
                                "symbol": "Z$"
                            }
                        },
                        "is_multi_currency": true
                    }
                }
            });

            let _ = fs::write(test_file, test_data.to_string());
            
            tokio::time::timeout(
                Duration::from_secs(5),
                data.ensure_initialized()
            ).await.expect("Initialization timed out");

            let is_zimbabwe_multi = data.is_multi_currency("Zimbabwe").await;
            let zimbabwe_currencies = data.get_available_currencies("Zimbabwe").await;
            
            assert!(is_zimbabwe_multi, 
                "Zimbabwe should be multi-currency. Available currencies: {:?}", 
                zimbabwe_currencies
            );

            if let Some(currencies) = zimbabwe_currencies {
                assert!(currencies.contains_key("USD"), "Should have USD");
                assert!(currencies.contains_key("ZWL"), "Should have ZWL");
            }

            cleanup_test_environment(&data).await;
            Ok::<(), Box<dyn std::error::Error>>(())
        }).await.unwrap();
    }
        
    #[tokio::test]
    async fn test_data_persistence() {
        let data = setup_test_environment().await;
        
        with_timeout(async {
            tokio::time::timeout(
                Duration::from_secs(5),
                data.ensure_initialized()
            ).await.expect("Initialization timed out");

            let test_file = Path::new("config/test/country_currencies.json");
            assert!(test_file.exists());

            let data2 = GlobalData::new("test_key".to_string());
            tokio::time::timeout(
                Duration::from_secs(5),
                data2.ensure_initialized()
            ).await.expect("Initialization timed out");
            
            let countries1 = data.countries.read().await;
            let countries2 = data2.countries.read().await;
            
            assert_eq!(countries1.len(), countries2.len());

            cleanup_test_environment(&data).await;
            cleanup_test_environment(&data2).await;
        }).await;
    }
}