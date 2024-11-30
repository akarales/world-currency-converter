use async_trait::async_trait;
use crate::errors::ServiceError;
use crate::models::{CountryInfo, CountryName, CurrencyInfo, ExchangeRateResponse};
use log::{debug, error};
use std::time::Duration;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct RestCountryResponse {
    name: RestCountryName,
    currencies: Option<HashMap<String, RestCurrencyInfo>>,
}

#[derive(Debug, Deserialize)]
struct RestCountryName {
    common: String,
    official: String,
}

#[derive(Debug, Deserialize)]
struct RestCurrencyInfo {
    name: String,
    symbol: String,
}

#[async_trait]
pub trait CountryClient: Send + Sync {
    async fn get_country_info(&self, country_name: &str) -> Result<CountryInfo, ServiceError>;
}

#[async_trait]
pub trait ExchangeRateClient: Send + Sync {
    async fn get_exchange_rate(
        &self,
        from_currency: &str,
    ) -> Result<ExchangeRateResponse, ServiceError>;
}

pub struct HttpClient {
    client: reqwest::Client,
    api_key: String,
}

impl HttpClient {
    pub fn new(client: reqwest::Client, api_key: String) -> Self {
        Self { client, api_key }
    }

    pub fn with_timeouts(timeout: Duration, api_key: String) -> Result<Self, ServiceError> {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .connect_timeout(timeout)
            .build()
            .map_err(|e| ServiceError::ConfigError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { client, api_key })
    }
}

#[async_trait]
impl CountryClient for HttpClient {
    async fn get_country_info(&self, country_name: &str) -> Result<CountryInfo, ServiceError> {
        let encoded_name = urlencoding::encode(country_name);
        let url = format!(
            "https://restcountries.com/v3.1/name/{}?fields=name,currencies",
            encoded_name
        );
        
        debug!("Fetching country info for: {}", country_name);
        let response = self.client.get(&url).send().await.map_err(|e| {
            error!("Failed to fetch country info: {}", e);
            ServiceError::ExternalApiError(format!("REST Countries API request failed: {}", e))
        })?;

        match response.status() {
            reqwest::StatusCode::OK => (),
            reqwest::StatusCode::NOT_FOUND => {
                debug!("Country not found: {}", country_name);
                return Err(ServiceError::CountryNotFound(country_name.to_string()));
            }
            status => {
                error!("REST Countries API error: {} for country: {}", status, country_name);
                return Err(ServiceError::ExternalApiError(format!(
                    "REST Countries API returned status: {}", 
                    status
                )));
            }
        }

        let countries: Vec<RestCountryResponse> = response
            .json()
            .await
            .map_err(|e| {
                error!("Failed to parse REST Countries API response: {}", e);
                ServiceError::ExternalApiError(format!("Failed to parse country data: {}", e))
            })?;

        let country = countries
            .into_iter()
            .next()
            .ok_or_else(|| ServiceError::CountryNotFound(country_name.to_string()))?;

        Ok(CountryInfo {
            name: CountryName {
                common: country.name.common,
                official: country.name.official,
                native_name: None,
            },
            currencies: country.currencies.map(|curr| {
                curr.into_iter()
                    .map(|(code, details)| {
                        (code, CurrencyInfo {
                            name: details.name,
                            symbol: details.symbol,
                        })
                    })
                    .collect()
            }),
        })
    }
}

#[async_trait]
impl ExchangeRateClient for HttpClient {
    async fn get_exchange_rate(&self, from_currency: &str) -> Result<ExchangeRateResponse, ServiceError> {
        let url = format!(
            "https://v6.exchangerate-api.com/v6/{}/latest/{}",
            self.api_key, from_currency
        );
        
        debug!("Fetching exchange rates for currency: {}", from_currency);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to fetch exchange rates: {}", e);
                ServiceError::ExternalApiError(format!("Exchange rate API request failed: {}", e))
            })?;

        match response.status() {
            reqwest::StatusCode::OK => (),
            reqwest::StatusCode::TOO_MANY_REQUESTS => {
                error!("Exchange rate API rate limit exceeded");
                return Err(ServiceError::RateLimitExceeded);
            }
            status => {
                error!("Exchange rate API error: {} for currency: {}", status, from_currency);
                return Err(ServiceError::ServiceUnavailable(
                    format!("Exchange rate service returned status: {}", status)
                ));
            }
        }

        let rate_response = response
            .json()
            .await
            .map_err(|e| {
                error!("Failed to parse exchange rate data: {}", e);
                ServiceError::ExternalApiError(format!("Failed to parse exchange rate data: {}", e))
            })?;

        debug!("Successfully fetched exchange rates for: {}", from_currency);
        
        Ok(rate_response)
    }
}

#[cfg(test)]
pub mod testing {
    use super::*;

    pub fn create_test_country_info(
        common_name: &str,
        currency_code: &str,
        currency_name: &str,
        currency_symbol: &str,
    ) -> CountryInfo {
        let mut currencies = HashMap::new();
        currencies.insert(
            currency_code.to_string(),
            CurrencyInfo {
                name: currency_name.to_string(),
                symbol: currency_symbol.to_string(),
            },
        );

        CountryInfo {
            name: CountryName {
                common: common_name.to_string(),
                official: format!("Official {}", common_name),
                native_name: None,
            },
            currencies: Some(currencies),
        }
    }

    // Rest of testing code...
}