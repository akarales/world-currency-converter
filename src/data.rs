use std::sync::Arc;
use tokio::sync::{RwLock, OnceCell};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use log::{debug, info, error};
use crate::update_service::UpdateService;
use crate::errors::ServiceError;
use crate::models::CurrencyInfo;
use crate::currency_manager::CurrencyManager;

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
        let client = reqwest::Client::new();
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
        let mut initialized = self.initialized.write().await;
        if !*initialized {
            self.update_service.start().await;
            
            if let Err(e) = self.load_data().await {
                error!("Failed to initialize global data: {}", e);
            }
            *initialized = true;
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
        
        // Use currency manager to get data
        match self.currency_manager.ensure_currency_data().await {
            Ok(currency_data) => {
                let mut countries_map = self.countries.write().await;
                let mut currencies_map = self.currencies.write().await;

                for (country_name, config) in currency_data.data {
                    let country_data = CountryData {
                        name: country_name.clone(),
                        code: country_name.clone(),
                        currencies: config.currencies.clone(),
                        primary_currency: config.primary_currency.clone(),
                        is_multi_currency: config.is_multi_currency,
                    };

                    countries_map.insert(country_name, country_data);

                    // Add currencies to global currency map
                    for (code, info) in config.currencies {
                        currencies_map.insert(code, info);
                    }
                }

                debug!(
                    "Loaded {} countries and {} currencies", 
                    countries_map.len(),
                    currencies_map.len()
                );
                Ok(())
            },
            Err(e) => {
                error!("Failed to load currency data: {}", e);
                Err(e)
            }
        }
    }
}

pub static GLOBAL_DATA: OnceCell<GlobalData> = OnceCell::const_new();

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tokio;

    #[tokio::test]
    async fn test_initialization() {
        let data = GlobalData::new("test_key".to_string());
        assert!(!*data.initialized.read().await);
        data.ensure_initialized().await;
        assert!(*data.initialized.read().await);
    }

    #[tokio::test]
    async fn test_multi_currency_detection() {
        let data = GlobalData::new("test_key".to_string());
        
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
    }

    #[tokio::test]
    async fn test_currency_loading() {
        let data = GlobalData::new("test_key".to_string());
        data.ensure_initialized().await;

        // Verify test config file exists
        let test_file = Path::new("config/test/country_currencies.json");
        assert!(test_file.exists());

        // Verify some known currencies are loaded
        let currencies = data.currencies.read().await;
        assert!(currencies.contains_key("USD"));
        assert!(currencies.contains_key("EUR"));
        
        // Verify some known multi-currency countries
        assert!(data.is_multi_currency("Panama").await);
        assert!(data.is_multi_currency("Zimbabwe").await);
    }

    #[tokio::test]
    async fn test_data_persistence() {
        let data = GlobalData::new("test_key".to_string());
        data.ensure_initialized().await;

        // Verify data is saved to test config
        let test_file = Path::new("config/test/country_currencies.json");
        assert!(test_file.exists());

        // Verify data can be reloaded
        let data2 = GlobalData::new("test_key".to_string());
        data2.ensure_initialized().await;
        
        let countries1 = data.countries.read().await;
        let countries2 = data2.countries.read().await;
        assert_eq!(countries1.len(), countries2.len());
    }
}