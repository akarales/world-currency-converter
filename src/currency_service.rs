use crate::{
    models::*,
    errors::ServiceError,
    clients::{CountryClient, ExchangeRateClient},
    cache::{Cache, ExchangeRateData}
};
use chrono::{DateTime, Utc};
use log::{debug, error, info};
use std::sync::Arc;
use uuid::Uuid;

pub struct CurrencyService<C>
where
    C: CountryClient + ExchangeRateClient,
{
    client: C,
    cache: Arc<Cache<ExchangeRateData>>,
}

impl<C> CurrencyService<C>
where
    C: CountryClient + ExchangeRateClient,
{
    pub fn new(client: C, cache: Arc<Cache<ExchangeRateData>>) -> Self {
        Self { client, cache }
    }

    pub async fn convert_currency(
        &self,
        request: &ConversionRequest,
    ) -> Result<DetailedConversionResponse, ServiceError> {
        let start_time = std::time::Instant::now();
        let request_id = Uuid::new_v4().to_string();

        debug!("Processing conversion request: {:?}", request);

        // Get source country details
        let from_country = self.client.get_country_info(&request.from).await?;
        let from_currencies = Self::get_available_currencies(&from_country);

        // Get destination country details
        let to_country = self.client.get_country_info(&request.to).await?;
        let to_currencies = Self::get_available_currencies(&to_country);

        // Select appropriate currencies
        let from_currency = self.select_currency(&from_currencies, request.preferred_currency.as_deref())?;
        let to_currency = self.select_currency(&to_currencies, request.preferred_currency.as_deref())?;

        // Get exchange rate
        let (converted_amount, rate, last_updated) = self.get_conversion_rate(
            &from_currency.code,
            &to_currency.code,
            request.amount,
        ).await?;

        info!(
            "Conversion successful: {} {} -> {} {} (rate: {})",
            request.amount,
            from_currency.code,
            converted_amount,
            to_currency.code,
            rate
        );

        // Create combined available currencies list if needed
        let available_currencies = if from_currencies.len() > 1 || to_currencies.len() > 1 {
            let mut combined = Vec::new();
            combined.extend(from_currencies.iter().cloned());
            combined.extend(to_currencies.iter().cloned());
            Some(combined)
        } else {
            None
        };

        let multiple_currencies_available = from_currencies.len() > 1 || to_currencies.len() > 1;

        Ok(DetailedConversionResponse {
            request_id,
            timestamp: Utc::now(),
            data: ConversionData {
                from: CurrencyDetails {
                    country: from_country.name.common,
                    currency_code: from_currency.code.clone(),
                    currency_name: from_currency.name.clone(),
                    currency_symbol: from_currency.symbol.clone(),
                    amount: request.amount,
                    is_primary: from_currency.is_primary,
                },
                to: CurrencyDetails {
                    country: to_country.name.common,
                    currency_code: to_currency.code.clone(),
                    currency_name: to_currency.name.clone(),
                    currency_symbol: to_currency.symbol.clone(),
                    amount: converted_amount,
                    is_primary: to_currency.is_primary,
                },
                exchange_rate: rate,
                last_updated,
                available_currencies,
            },
            meta: ResponseMetadata {
                source: "exchangerate-api.com".to_string(),
                response_time_ms: start_time.elapsed().as_millis() as u64,
                multiple_currencies_available,
                cache_hit: None,  // TODO implement cache tracking
                rate_limit_remaining: None,  // TODO implement rate limiting
            },
        })
    }

    fn get_available_currencies(country: &CountryInfo) -> Vec<AvailableCurrency> {
        let is_multi_currency = country.currencies.len() > 1;
        country
            .currencies
            .iter()
            .map(|(code, info)| AvailableCurrency {
                code: code.clone(),
                name: info.name.clone(),
                symbol: info.symbol.clone(),
                is_primary: !is_multi_currency || code == "USD" || code == "EUR",
            })
            .collect()
    }

    fn select_currency<'a>(
        &self,
        currencies: &'a [AvailableCurrency],
        preferred: Option<&str>,
    ) -> Result<&'a AvailableCurrency, ServiceError> {
        match (preferred, currencies.len()) {
            (Some(preferred), _) => currencies
                .iter()
                .find(|c| c.code == preferred)
                .ok_or_else(|| ServiceError::InvalidCurrency(format!("Preferred currency {} not available", preferred))),
            (None, 1) => Ok(&currencies[0]),
            (None, _) => currencies
                .iter()
                .find(|c| c.is_primary)
                .ok_or_else(|| ServiceError::InvalidCurrency("No primary currency found".to_string())),
        }
    }

    async fn get_conversion_rate(
        &self,
        from_currency: &str,
        to_currency: &str,
        amount: f64,
    ) -> Result<(f64, f64, DateTime<Utc>), ServiceError> {
        // Check cache first
        let cache_key = format!("{}_{}", from_currency, to_currency);
        if let Some(cached) = self.cache.get(&cache_key).await {
            debug!("Cache hit for {}->{}", from_currency, to_currency);
            let rate = cached.rate;
            let converted_amount = (amount * rate * 100.0).round() / 100.0;
            return Ok((converted_amount, rate, cached.last_updated));
        }

        // Get fresh rates from API
        let response = self.client.get_exchange_rate(from_currency).await?;
        let now = Utc::now();

        let rate = response.conversion_rates
            .get(to_currency)
            .ok_or_else(|| {
                error!("Exchange rate not found for {}->{}", from_currency, to_currency);
                ServiceError::InvalidCurrency(format!("Exchange rate not found for {}->{}", from_currency, to_currency))
            })?;

        let converted_amount = (amount * rate * 100.0).round() / 100.0;

        // Cache the result
        self.cache.set(
            cache_key,
            ExchangeRateData {
                rate: *rate,
                last_updated: now,
            },
        ).await;
        
        debug!(
            "Exchange rate lookup successful: {} {} = {} {} (rate: {})",
            amount, from_currency, converted_amount, to_currency, rate
        );
        
        Ok((converted_amount, *rate, now))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::tests::{MockClient, create_test_country_info};
    use std::collections::HashMap;

    fn create_mock_exchange_rate_response(base: &str, rates: &[(&str, f64)]) -> ExchangeRateResponse {
        let mut conversion_rates = HashMap::new();
        // Always include base currency with rate 1.0
        conversion_rates.insert(base.to_string(), 1.0);
        for (currency, rate) in rates {
            conversion_rates.insert(currency.to_string(), *rate);
        }

        ExchangeRateResponse {
            result: "success".to_string(),
            conversion_rates,
            time_last_update_utc: Some("2024-01-01".to_string()),
        }
    }

    #[tokio::test]
    async fn test_basic_conversion() {
        // Setup
        let from_country = create_test_country_info(
            "United States", "USD", "US Dollar", "$"
        );
        let to_country = create_test_country_info(
            "France", "EUR", "Euro", "€"
        );
        let exchange_rates = create_mock_exchange_rate_response(
            "USD",
            &[("EUR", 0.85)]
        );

        let mock_client = MockClient::new()
            .with_country_response(from_country.clone())
            .with_country_response(to_country)
            .with_rate_response(exchange_rates);

        let cache = Arc::new(Cache::new(60, 100));
        let service = CurrencyService::new(mock_client, cache);

        // Test
        let request = ConversionRequest {
            from: "United States".to_string(),
            to: "France".to_string(),
            amount: 100.0,
            preferred_currency: None,
        };

        let result = service.convert_currency(&request).await.unwrap();

        // Assert
        assert_eq!(result.data.from.currency_code, "USD");
        assert_eq!(result.data.to.currency_code, "EUR");
        assert_eq!(result.data.exchange_rate, 0.85);
        assert_eq!(result.data.to.amount, 85.0);
        
        // Also test same currency conversion
        let same_currency_request = ConversionRequest {
            from: "United States".to_string(),
            to: "United States".to_string(),
            amount: 100.0,
            preferred_currency: None,
        };

        let result = service.convert_currency(&same_currency_request).await.unwrap();
        assert_eq!(result.data.from.currency_code, "USD");
        assert_eq!(result.data.to.currency_code, "USD");
        assert_eq!(result.data.exchange_rate, 1.0);
        assert_eq!(result.data.to.amount, 100.0);
    }
}