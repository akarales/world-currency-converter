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
            "United States" => {
                currencies.insert(
                    "USD".to_string(),
                    CurrencyInfo {
                        name: "US Dollar".to_string(),
                        symbol: "$".to_string(),
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

        if from == "INVALID" {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_country_client() {
        let client = MockHttpClient::default();
        
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
        let client = MockHttpClient::default();

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