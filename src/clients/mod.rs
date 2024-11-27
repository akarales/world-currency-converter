use async_trait::async_trait;
use crate::errors::ServiceError;
use crate::models::{CountryInfo, ExchangeRateResponse};
use log::{debug, error};
use std::time::Duration;

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
        let url = format!(
            "https://restcountries.com/v3.1/name/{}?fields=name,currencies",
            urlencoding::encode(country_name)
        );
        
        debug!("Fetching country info for: {}", country_name);
        let response = self.client
            .get(&url)
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            debug!("Country not found: {}", country_name);
            return Err(ServiceError::CountryNotFound(country_name.to_string()));
        }

        if !response.status().is_success() {
            error!("Country API error: {} for country: {}", response.status(), country_name);
            return Err(ServiceError::ExternalApiError(format!(
                "Country API returned status: {}", 
                response.status()
            )));
        }

        let countries: Vec<CountryInfo> = response
            .json()
            .await
            .map_err(|e| {
                error!("Failed to parse country data for {}: {}", country_name, e);
                ServiceError::ExternalApiError(format!("Failed to parse country data: {}", e))
            })?;

        countries
            .into_iter()
            .next()
            .ok_or_else(|| ServiceError::CountryNotFound(country_name.to_string()))
    }
}

#[async_trait]
impl ExchangeRateClient for HttpClient {
    async fn get_exchange_rate(
        &self,
        from_currency: &str,
    ) -> Result<ExchangeRateResponse, ServiceError> {
        let url = format!(
            "https://v6.exchangerate-api.com/v6/{}/latest/{}",
            self.api_key, from_currency
        );
        
        debug!("Fetching exchange rates for: {}", from_currency);
        let response = self.client
            .get(&url)
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            error!("Exchange rate API rate limit exceeded");
            return Err(ServiceError::RateLimitExceeded);
        }

        if !response.status().is_success() {
            error!("Exchange rate API error: {} for currency: {}", response.status(), from_currency);
            return Err(ServiceError::ServiceUnavailable(
                "Exchange rate service unavailable".to_string()
            ));
        }

        response
            .json()
            .await
            .map_err(|e| {
                error!("Failed to parse exchange rate data: {}", e);
                ServiceError::ExternalApiError(format!("Failed to parse exchange rate data: {}", e))
            })
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::collections::HashMap;

    pub struct MockClient {
        pub country_response: Option<CountryInfo>,
        pub rate_response: Option<ExchangeRateResponse>,
    }

    impl MockClient {
        pub fn new() -> Self {
            Self {
                country_response: None,
                rate_response: None,
            }
        }

        pub fn with_country_response(mut self, response: CountryInfo) -> Self {
            self.country_response = Some(response);
            self
        }

        pub fn with_rate_response(mut self, response: ExchangeRateResponse) -> Self {
            self.rate_response = Some(response);
            self
        }
    }

    #[async_trait]
    impl CountryClient for MockClient {
        async fn get_country_info(&self, country_name: &str) -> Result<CountryInfo, ServiceError> {
            self.country_response.clone()
                .ok_or_else(|| ServiceError::CountryNotFound(country_name.to_string()))
        }
    }

    #[async_trait]
    impl ExchangeRateClient for MockClient {
        async fn get_exchange_rate(
            &self,
            from_currency: &str,
        ) -> Result<ExchangeRateResponse, ServiceError> {
            self.rate_response.clone()
                .ok_or_else(|| ServiceError::ServiceUnavailable(
                    format!("No mock response for currency: {}", from_currency)
                ))
        }
    }

    // Helper function to create test data
    pub fn create_test_country_info(
        common_name: &str,
        currency_code: &str,
        currency_name: &str,
        currency_symbol: &str,
    ) -> CountryInfo {
        let mut currencies = HashMap::new();
        currencies.insert(
            currency_code.to_string(),
            crate::models::CurrencyInfo {
                name: currency_name.to_string(),
                symbol: currency_symbol.to_string(),
            },
        );

        CountryInfo {
            name: crate::models::CountryName {
                common: common_name.to_string(),
                official: format!("Official {}", common_name),
            },
            currencies,
        }
    }

    #[cfg(test)]
    mod client_tests {
        use super::*;
        use tokio;

        #[tokio::test]
        async fn test_mock_client() {
            let test_country = create_test_country_info(
                "Test Country",
                "TST",
                "Test Currency",
                "T$",
            );

            let mock = MockClient::new()
                .with_country_response(test_country.clone());

            let result = mock.get_country_info("Test Country").await.unwrap();
            assert_eq!(result.name.common, "Test Country");
        }
    }
}