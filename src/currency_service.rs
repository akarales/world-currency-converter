use crate::{
    models::*,
    errors::ServiceError,
    clients::{CountryClient, ExchangeRateClient},
    cache::{RateCache, ExchangeRateData},
    data::GLOBAL_DATA
};
use chrono::{DateTime, Utc};
use log::debug;
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

pub struct CurrencyService<C>
where
    C: CountryClient + ExchangeRateClient,
{
    client: C,
    cache: Arc<RateCache>,
}

impl<C> CurrencyService<C>
where
    C: CountryClient + ExchangeRateClient,
{
    pub fn new(client: C, cache: Arc<RateCache>) -> Self {
        Self { client, cache }
    }

    pub async fn convert_currency(
        &self,
        request: &ConversionRequest,
    ) -> Result<DetailedConversionResponse, ServiceError> {
        let start_time = std::time::Instant::now();
        let request_id = Uuid::new_v4().to_string();
    
        debug!("Processing conversion request: {:?}", request);
    
        // Get country information first
        let from_country = self.client.get_country_info(&request.from).await?;
        let to_country = self.client.get_country_info(&request.to).await?;
    
        // Get currencies with proper error handling
        let from_currencies = match &from_country.currencies {
            Some(currencies) => currencies,
            None => return Err(ServiceError::InvalidCurrency(
                format!("No currencies found for {}", request.from)
            )),
        };

        let to_currencies = match &to_country.currencies {
            Some(currencies) => currencies,
            None => return Err(ServiceError::InvalidCurrency(
                format!("No currencies found for {}", request.to)
            )),
        };

        // Store primary currency values
        let from_primary = GLOBAL_DATA.get().unwrap().get_primary_currency(&request.from).await;
        let to_primary = GLOBAL_DATA.get().unwrap().get_primary_currency(&request.to).await;

        // Check multi-currency status
        let from_multi = GLOBAL_DATA.get().unwrap().is_multi_currency(&request.from).await;
        let to_multi = GLOBAL_DATA.get().unwrap().is_multi_currency(&to_country.name.common).await;
        let multiple_currencies_available = from_multi || to_multi;

        // Prepare available currencies if needed
        let available_currencies = if multiple_currencies_available {
            let mut currencies = Vec::new();
            
            // Add source country currencies
            for (code, info) in from_currencies {
                let is_primary = from_primary.as_deref() == Some(code);
                currencies.push(AvailableCurrency {
                    code: code.clone(),
                    name: info.name.clone(),
                    symbol: info.symbol.clone(),
                    is_primary,
                });
            }

            // Add destination country currencies if different
            for (code, info) in to_currencies {
                if !currencies.iter().any(|c| &c.code == code) {
                    let is_primary = to_primary.as_deref() == Some(code);
                    currencies.push(AvailableCurrency {
                        code: code.clone(),
                        name: info.name.clone(),
                        symbol: info.symbol.clone(),
                        is_primary,
                    });
                }
            }
            Some(currencies)
        } else {
            None
        };

        // Select currencies with proper lifetimes
        let (from_currency_code, from_currency_info) = self.select_currency(
            from_currencies,
            request.preferred_currency.as_deref(),
            from_primary.as_deref(),
            &request.from,
        )?;

        let (to_currency_code, to_currency_info) = self.select_currency(
            to_currencies,
            request.preferred_currency.as_deref(),
            to_primary.as_deref(),
            &request.to,
        )?;

        // Handle same currency case
        if from_currency_code == to_currency_code {
            return Ok(DetailedConversionResponse {
                request_id,
                timestamp: Utc::now(),
                data: ConversionData {
                    from: CurrencyDetails {
                        country: from_country.name.common.clone(),
                        currency_code: from_currency_code.to_string(),
                        currency_name: from_currency_info.name.clone(),
                        currency_symbol: from_currency_info.symbol.clone(),
                        amount: request.amount,
                        is_primary: from_primary.as_deref() == Some(from_currency_code),
                    },
                    to: CurrencyDetails {
                        country: to_country.name.common.clone(),
                        currency_code: to_currency_code.to_string(),
                        currency_name: to_currency_info.name.clone(),
                        currency_symbol: to_currency_info.symbol.clone(),
                        amount: request.amount,
                        is_primary: to_primary.as_deref() == Some(to_currency_code),
                    },
                    exchange_rate: 1.0,
                    last_updated: Utc::now(),
                    available_currencies,
                },
                meta: ResponseMetadata {
                    source: "exchangerate-api.com".to_string(),
                    response_time_ms: start_time.elapsed().as_millis() as u64,
                    multiple_currencies_available,
                    cache_hit: Some(true),
                    rate_limit_remaining: None,
                },
            });
        }

        // Get conversion rate and perform conversion
        let (converted_amount, rate, last_updated, cache_hit) = self.get_conversion_rate(
            from_currency_code,
            to_currency_code,
            request.amount,
        ).await?;

        Ok(DetailedConversionResponse {
            request_id,
            timestamp: Utc::now(),
            data: ConversionData {
                from: CurrencyDetails {
                    country: from_country.name.common,
                    currency_code: from_currency_code.to_string(),
                    currency_name: from_currency_info.name.clone(),
                    currency_symbol: from_currency_info.symbol.clone(),
                    amount: request.amount,
                    is_primary: from_primary.as_deref() == Some(from_currency_code),
                },
                to: CurrencyDetails {
                    country: to_country.name.common,
                    currency_code: to_currency_code.to_string(),
                    currency_name: to_currency_info.name.clone(),
                    currency_symbol: to_currency_info.symbol.clone(),
                    amount: converted_amount,
                    is_primary: to_primary.as_deref() == Some(to_currency_code),
                },
                exchange_rate: rate,
                last_updated,
                available_currencies,
            },
            meta: ResponseMetadata {
                source: "exchangerate-api.com".to_string(),
                response_time_ms: start_time.elapsed().as_millis() as u64,
                multiple_currencies_available,
                cache_hit: Some(cache_hit),
                rate_limit_remaining: None,
            },
        })
    }

    fn select_currency<'a>(
        &self,
        currencies: &'a HashMap<String, CurrencyInfo>,
        preferred: Option<&'a str>,
        primary: Option<&'a str>,
        country_name: &str,
    ) -> Result<(&'a str, &'a CurrencyInfo), ServiceError> {
        if let Some(pref) = preferred {
            if let Some(info) = currencies.get(pref) {
                return Ok((pref, info));
            } else {
                return Err(ServiceError::InvalidCurrency(
                    format!("Preferred currency {} not available for {}. Available currencies: {}", 
                        pref,
                        country_name,
                        currencies.keys().cloned().collect::<Vec<_>>().join(", ")
                    )
                ));
            }
        }

        if let Some(primary_code) = primary {
            if let Some(info) = currencies.get(primary_code) {
                return Ok((primary_code, info));
            }
        }

        for code in ["USD", "EUR"] {
            if let Some(info) = currencies.get(code) {
                return Ok((code, info));
            }
        }

        currencies
            .iter()
            .next()
            .map(|(code, info)| (code.as_str(), info))
            .ok_or_else(|| ServiceError::InvalidCurrency(
                format!("No valid currency found for {}", country_name)
            ))
    }

    async fn get_conversion_rate(
        &self,
        from_currency: &str,
        to_currency: &str,
        amount: f64,
    ) -> Result<(f64, f64, DateTime<Utc>, bool), ServiceError> {
        let cache_key = format!("{}_{}", from_currency, to_currency);
        
        if let Some(cached) = self.cache.get(&cache_key).await {
            debug!("Cache hit for {}->{}", from_currency, to_currency);
            let converted_amount = (amount * cached.rate * 100.0).round() / 100.0;
            return Ok((converted_amount, cached.rate, cached.last_updated, true));
        }

        let response = self.client.get_exchange_rate(from_currency).await?;
        let now = Utc::now();

        let rate = response.conversion_rates
            .get(to_currency)
            .ok_or_else(|| {
                ServiceError::InvalidCurrency(format!(
                    "Exchange rate not found for {}->{}", 
                    from_currency, 
                    to_currency
                ))
            })?;

        let converted_amount = (amount * rate * 100.0).round() / 100.0;

        self.cache.set(
            cache_key,
            ExchangeRateData {
                rate: *rate,
                last_updated: now,
            },
        ).await;
        
        Ok((converted_amount, *rate, now, false))
    }
}