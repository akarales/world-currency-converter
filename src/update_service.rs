use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use log::{info, error};
use reqwest::Client;
use std::fs;
use std::path::Path;
use crate::models::{CountryInfo, CurrencyInfo};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyUpdateInfo {
    pub last_checked: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
    pub data: HashMap<String, CountryCurrencyConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountryCurrencyConfig {
    pub primary_currency: String,
    pub currencies: HashMap<String, CurrencyInfo>,
    pub is_multi_currency: bool,
}

#[derive(Debug)]
pub struct UpdateService {
    client: Client,
    currency_data: Arc<RwLock<CurrencyUpdateInfo>>,
    api_key: String,
    config_path: String,
}

impl UpdateService {
    pub fn new(api_key: String) -> Self {
        let config_path = "../config/country_currencies.json ".to_string();
        
        let currency_data = Arc::new(RwLock::new(CurrencyUpdateInfo {
            last_checked: Utc::now(),
            last_modified: Utc::now(),
            data: HashMap::new(),
        }));

        Self {
            client: Client::new(),
            currency_data,
            api_key,
            config_path,
        }
    }

    pub async fn start(&self) {
        // Load initial data from file
        self.load_from_file().await;
        
        // Clone references for the background task
        let client = self.client.clone();
        let currency_data = Arc::clone(&self.currency_data);
        let api_key = self.api_key.clone();
        let config_path = self.config_path.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
                let now = Utc::now();
                let needs_update = {
                    let data = currency_data.read().await;
                    now.signed_duration_since(data.last_checked).num_hours() >= 24
                };

                if needs_update {
                    info!("Starting daily currency data update check");
                    if let Err(e) = Self::update_data(&client, &currency_data, &config_path).await {
                        error!("Failed to update currency data: {}", e);
                    }
                }
            }
        });
    }

    async fn load_from_file(&self) {
        if let Ok(content) = fs::read_to_string(&self.config_path) {
            match serde_json::from_str::<CurrencyUpdateInfo>(&content) {
                Ok(config) => {
                    let mut current_data = self.currency_data.write().await;
                    *current_data = config;
                    info!("Loaded currency data from file with {} entries", current_data.data.len());
                }
                Err(e) => {
                    error!("Failed to parse currency config file: {}", e);
                }
            }
        }
    }

    pub async fn update_data(
        client: &Client,
        currency_data: &Arc<RwLock<CurrencyUpdateInfo>>,
        config_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Checking for currency data updates...");
    
        let response = client
            .get("https://restcountries.com/v3.1/all?fields=name,currencies")
            .send()
            .await?;

        let countries: Vec<CountryInfo> = response.json().await?;
        let mut updated_data = HashMap::new();
        let now = Utc::now();

        for country in countries {
            if let Some(currencies) = country.currencies {
                let is_multi_currency = currencies.len() > 1;
                
                // Determine primary currency
                let primary_currency = if currencies.contains_key("USD") {
                    "USD".to_string()
                } else if currencies.contains_key("EUR") {
                    "EUR".to_string()
                } else {
                    currencies.keys().next().unwrap().clone()
                };

                let config = CountryCurrencyConfig {
                    primary_currency,
                    currencies,
                    is_multi_currency,
                };

                updated_data.insert(country.name.common.clone(), config);
            }
        }

        let update_info = CurrencyUpdateInfo {
            last_checked: now,
            last_modified: now,
            data: updated_data,
        };

        // Ensure config directory exists
        if let Some(parent) = Path::new(config_path).parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
    
        let json = serde_json::to_string_pretty(&update_info)?;
        fs::write(config_path, json)?;
    
        let mut current_data = currency_data.write().await;
        *current_data = update_info;
    
        info!("Currency data updated successfully");
        Ok(())
    }

    pub async fn get_currency_config(&self, country: &str) -> Option<CountryCurrencyConfig> {
        self.currency_data.read().await.data.get(country).cloned()
    }

    pub async fn get_exchange_rate(&self, from: &str, to: &str) -> Option<f64> {
        let data = self.currency_data.read().await;
        data.data.values()
            .find(|config| config.primary_currency == from)
            .and_then(|config| config.currencies.get(to))
            .map(|info| info.symbol.parse().unwrap_or(1.0))
    }

    pub async fn get_currency_info(&self, country: &str, currency_code: &str) -> Option<(String, String)> {
        let data = self.currency_data.read().await;
        data.data.get(country).and_then(|config| {
            config.currencies.get(currency_code).map(|info| {
                (info.name.clone(), info.symbol.clone())
            })
        })
    }
}