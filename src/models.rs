use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct ConversionRequest {
    pub from: String,
    pub to: String,
    pub amount: f64,
    pub preferred_currency: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SimpleConversionResponse {
    pub from: String,
    pub to: String,
    pub amount: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CountryInfo {
    pub name: CountryName,
    pub currencies: HashMap<String, CurrencyInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CountryName {
    pub common: String,
    pub official: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CurrencyInfo {
    pub name: String,
    pub symbol: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DetailedConversionResponse {
    pub request_id: String,
    pub timestamp: DateTime<Utc>,
    pub data: ConversionData,
    pub meta: ResponseMetadata,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConversionData {
    pub from: CurrencyDetails,
    pub to: CurrencyDetails,
    pub exchange_rate: f64,
    pub last_updated: DateTime<Utc>,
    pub available_currencies: Option<Vec<AvailableCurrency>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CurrencyDetails {
    pub country: String,
    pub currency_code: String,
    pub currency_name: String,
    pub currency_symbol: String,
    pub amount: f64,
    pub is_primary: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AvailableCurrency {
    pub code: String,
    pub name: String,
    pub symbol: String,
    pub is_primary: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResponseMetadata {
    pub source: String,
    pub response_time_ms: u64,
    pub multiple_currencies_available: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DetailedErrorResponse {
    pub error: String,
    pub request_id: String,
    pub timestamp: DateTime<Utc>,
    pub available_currencies: Option<Vec<AvailableCurrency>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExchangeRateResponse {
    pub result: String,
    pub conversion_rates: HashMap<String, f64>,
    pub time_last_update_utc: Option<String>,
}

#[derive(Debug)]
pub struct AppError(pub String);

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for AppError {}