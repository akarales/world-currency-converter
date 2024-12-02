use async_trait::async_trait;
use crate::errors::ServiceError;
use crate::models::{CountryInfo, ExchangeRateResponse};
use reqwest::Client;
use std::time::Duration;
use log::debug;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct CountryApiResponse {
    pub name: CountryName,
    pub currencies: Option<HashMap<String, ApiCurrencyInfo>>,
}

#[derive(Debug, Deserialize)]
pub struct CountryName {
    pub common: String,
}

#[derive(Debug, Deserialize)]
pub struct ApiCurrencyInfo {
    pub name: String,
    pub symbol: String,
}

#[async_trait]
pub trait CountryClient: Send + Sync {
    async fn get_country_info(&self, country_name: &str) -> Result<CountryInfo, ServiceError>;
}

#[async_trait]
pub trait ExchangeRateClient: Send + Sync {
    async fn get_exchange_rate(&self, from: &str) -> Result<ExchangeRateResponse, ServiceError>;
}

#[derive(Clone)]
pub struct HttpClient {
    client: Client,
    api_key: String,
}

impl HttpClient {
    pub fn new(client: Client, api_key: String) -> Self {
        Self { client, api_key }
    }

    pub fn with_timeouts(timeout: Duration, api_key: String) -> Result<Self, reqwest::Error> {
        let client = Client::builder()
            .timeout(timeout)
            .build()?;
        Ok(Self::new(client, api_key))
    }
}

#[async_trait]
impl CountryClient for HttpClient {
    async fn get_country_info(&self, country_name: &str) -> Result<CountryInfo, ServiceError> {
        debug!("Fetching country info for: {}", country_name);
        
        let url = format!(
            "https://restcountries.com/v3.1/name/{}?fields=name,currencies",
            urlencoding::encode(country_name)
        );

        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| ServiceError::ExternalApiError(format!("Failed to fetch country data: {}", e)))?;

        if response.status().as_u16() == 404 {
            return Err(ServiceError::CountryNotFound(country_name.to_string()));
        }

        let countries: Vec<CountryInfo> = response
            .json()
            .await
            .map_err(|e| ServiceError::ExternalApiError(format!("Failed to parse country data: {}", e)))?;

        countries.into_iter()
            .next()
            .ok_or_else(|| ServiceError::CountryNotFound(country_name.to_string()))
    }
}

#[async_trait]
impl ExchangeRateClient for HttpClient {
    async fn get_exchange_rate(&self, from_currency: &str) -> Result<ExchangeRateResponse, ServiceError> {
        debug!("Fetching exchange rate for currency: {}", from_currency);
        
        let url = format!(
            "https://v6.exchangerate-api.com/v6/{}/latest/{}",
            self.api_key,
            from_currency
        );

        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| ServiceError::ExternalApiError(format!("Failed to fetch exchange rates: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            return Err(ServiceError::ExternalApiError(
                format!("Exchange rate service returned status: {}", status)
            ));
        }

        response
            .json()
            .await
            .map_err(|e| ServiceError::ExternalApiError(format!("Failed to parse exchange rates: {}", e)))
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::models::CountryName as ModelCountryName;
    use crate::models::CurrencyInfo;

    pub struct MockHttpClient;

    #[async_trait]
    impl CountryClient for MockHttpClient {
        async fn get_country_info(&self, country_name: &str) -> Result<CountryInfo, ServiceError> {
            if country_name == "Narnia" {
                return Err(ServiceError::CountryNotFound(country_name.to_string()));
            }

            let mut currencies = HashMap::new();
            
            match country_name {
                "Zimbabwe" => {
                    currencies.insert(
                        "USD".to_string(),
                        CurrencyInfo {
                            name: "US Dollar".to_string(),
                            symbol: "$".to_string(),
                        }
                    );
                    currencies.insert(
                        "ZWL".to_string(),
                        CurrencyInfo {
                            name: "Zimbabwean Dollar".to_string(),
                            symbol: "Z$".to_string(),
                        }
                    );
                },
                "France" => {
                    currencies.insert(
                        "EUR".to_string(),
                        CurrencyInfo {
                            name: "Euro".to_string(),
                            symbol: "€".to_string(),
                        }
                    );
                },
                _ => {
                    currencies.insert(
                        "USD".to_string(),
                        CurrencyInfo {
                            name: "US Dollar".to_string(),
                            symbol: "$".to_string(),
                        }
                    );
                }
            }

            Ok(CountryInfo {
                name: ModelCountryName {
                    common: country_name.to_string(),
                    official: format!("Official {}", country_name),
                    native_name: None,
                },
                currencies: Some(currencies),
            })
        }
    }

    #[async_trait]
    impl ExchangeRateClient for MockHttpClient {
        async fn get_exchange_rate(&self, from: &str) -> Result<ExchangeRateResponse, ServiceError> {
            debug!("Mock: Getting exchange rate for {}", from);
            
            let mut rates = HashMap::new();
            rates.insert("USD".to_string(), 1.0);
            rates.insert("EUR".to_string(), 0.85);
            rates.insert("GBP".to_string(), 0.73);
            rates.insert("ZWL".to_string(), 322.0);

            if !rates.contains_key(from) {
                return Err(ServiceError::ExternalApiError(
                    format!("Currency not found: {}", from)
                ));
            }

            Ok(ExchangeRateResponse {
                result: "success".to_string(),
                conversion_rates: rates,
                time_last_update_utc: Some(chrono::Utc::now().to_rfc3339()),
            })
        }
    }

    #[tokio::test]
    async fn test_mock_country_client() {
        let client = MockHttpClient;
        
        // Test valid country
        let result = client.get_country_info("United States").await;
        assert!(result.is_ok());
        let country = result.unwrap();
        assert!(country.currencies.unwrap().contains_key("USD"));

        // Test Zimbabwe (multi-currency)
        let result = client.get_country_info("Zimbabwe").await;
        assert!(result.is_ok());
        let country = result.unwrap();
        let currencies = country.currencies.unwrap();
        assert!(currencies.contains_key("USD"));
        assert!(currencies.contains_key("ZWL"));

        // Test invalid country
        let result = client.get_country_info("Narnia").await;
        assert!(matches!(result, Err(ServiceError::CountryNotFound(_))));
    }

    #[tokio::test]
    async fn test_mock_exchange_rate_client() {
        let client = MockHttpClient;

        // Test valid currency
        let result = client.get_exchange_rate("USD").await;
        assert!(result.is_ok());
        let rates = result.unwrap();
        assert_eq!(rates.conversion_rates.get("EUR").unwrap(), &0.85);

        // Test invalid currency
        let result = client.get_exchange_rate("INVALID").await;
        assert!(matches!(result, Err(ServiceError::ExternalApiError(_))));
    }
}