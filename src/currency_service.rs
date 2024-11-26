use crate::models::*;
use chrono::{DateTime, Utc};
use log::{debug, error, info};
use reqwest::Client;
use std::env;
use uuid::Uuid;

pub struct CurrencyService {
    client: Client,
}

impl CurrencyService {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn get_country_currencies(&self, country_name: &str) -> Result<CountryInfo, AppError> {
        let url = format!(
            "https://restcountries.com/v3.1/name/{}?fields=name,currencies",
            urlencoding::encode(country_name)
        );
        
        debug!("Fetching country info for: {}", country_name);
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to connect to country service: {}", e);
                AppError(format!("Failed to connect to country service: {}", e))
            })?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            debug!("Country not found: {}", country_name);
            return Err(AppError(format!("Country not found: {}", country_name)));
        }

        let countries: Vec<CountryInfo> = response
            .json()
            .await
            .map_err(|e| {
                error!("Failed to parse country data for {}: {}", country_name, e);
                AppError(format!("Failed to parse country data: {}", e))
            })?;

        countries
            .into_iter()
            .next()
            .ok_or_else(|| AppError(format!("No data found for country: {}", country_name)))
    }

    pub async fn convert_currency(
        &self,
        request: &ConversionRequest,
    ) -> Result<DetailedConversionResponse, AppError> {
        let start_time = std::time::Instant::now();
        let request_id = Uuid::new_v4().to_string();

        debug!("Processing conversion request: {:?}", request);

        // Get source country details
        let from_country = self.get_country_currencies(&request.from).await?;
        let from_currencies = Self::get_available_currencies(&from_country);

        // Get destination country details
        let to_country = self.get_country_currencies(&request.to).await?;
        let to_currencies = Self::get_available_currencies(&to_country);

        // Select appropriate currencies
        let from_currency = self.select_currency(&from_currencies, request.preferred_currency.as_deref())?;
        let to_currency = self.select_currency(&to_currencies, request.preferred_currency.as_deref())?;

        // Get exchange rate
        let (converted_amount, rate, last_updated) = self.get_exchange_rate(
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
    ) -> Result<&'a AvailableCurrency, AppError> {
        match (preferred, currencies.len()) {
            (Some(preferred), _) => currencies
                .iter()
                .find(|c| c.code == preferred)
                .ok_or_else(|| AppError(format!("Preferred currency {} not available", preferred))),
            (None, 1) => Ok(&currencies[0]),
            (None, _) => currencies
                .iter()
                .find(|c| c.is_primary)
                .ok_or_else(|| AppError("No primary currency found".to_string())),
        }
    }

    async fn get_exchange_rate(
        &self,
        from_currency: &str,
        to_currency: &str,
        amount: f64,
    ) -> Result<(f64, f64, DateTime<Utc>), AppError> {
        let api_key = env::var("EXCHANGE_RATE_API_KEY")
            .map_err(|_| {
                error!("Exchange rate API key not found in environment");
                AppError("Exchange rate API key not found".into())
            })?;
        
        let url = format!(
            "https://v6.exchangerate-api.com/v6/{}/latest/{}",
            api_key, from_currency
        );
        
        debug!("Fetching exchange rate for {} -> {}", from_currency, to_currency);
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to connect to exchange rate service: {}", e);
                AppError(format!("Failed to connect to exchange rate service: {}", e))
            })?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            error!("Exchange rate API rate limit exceeded");
            return Err(AppError("Rate limit exceeded. Please try again later.".into()));
        }

        if !response.status().is_success() {
            error!("Exchange rate API error: {}", response.status());
            return Err(AppError("Failed to fetch exchange rates".into()));
        }

        let data: ExchangeRateResponse = response
            .json()
            .await
            .map_err(|e| {
                error!("Failed to parse exchange rate data: {}", e);
                AppError("Failed to parse exchange rate data".into())
            })?;

        let rate = data.conversion_rates
            .get(to_currency)
            .ok_or_else(|| {
                error!("Exchange rate not found for {}->{}", from_currency, to_currency);
                AppError(format!("Exchange rate not found for {}->{}", from_currency, to_currency))
            })?;

        let converted_amount = (amount * rate * 100.0).round() / 100.0;
        
        debug!(
            "Exchange rate lookup successful: {} {} = {} {} (rate: {})", 
            amount, from_currency, converted_amount, to_currency, rate
        );
        
        Ok((converted_amount, *rate, Utc::now()))
    }
}