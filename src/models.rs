use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub trait Validate {
    fn validate(&self) -> Result<(), String>;
}

#[derive(Debug, Clone, Deserialize)]
pub struct CountryInfo {
    pub name: CountryName,
    #[serde(default)]
    pub currencies: Option<HashMap<String, CurrencyInfo>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CountryName {
    pub common: String,
    pub official: String,
    #[serde(default)]
    pub native_name: Option<HashMap<String, NativeName>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NativeName {
    pub official: String,
    pub common: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyInfo {
    pub name: String,
    pub symbol: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CurrencyUpdateInfo {
    pub last_checked: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
    pub data: HashMap<String, CountryCurrencyConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CountryCurrencyConfig {
    pub primary_currency: String,
    pub currencies: HashMap<String, CurrencyInfo>,
    pub is_multi_currency: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ConversionRequest {
    pub from: String,
    pub to: String,
    pub amount: f64,
    pub preferred_currency: Option<String>,
}

impl Validate for ConversionRequest {
    fn validate(&self) -> Result<(), String> {
        if self.amount <= 0.0 {
            return Err("Amount must be greater than 0".to_string());
        }
        if self.from.trim().is_empty() {
            return Err("Source country cannot be empty".to_string());
        }
        if self.to.trim().is_empty() {
            return Err("Destination country cannot be empty".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SimpleConversionResponse {
    pub from: String,
    pub to: String,
    pub amount: f64,
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
    #[serde(skip_serializing_if = "Option::is_none")]
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_remaining: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_hit: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExchangeRateResponse {
    pub result: String,
    pub conversion_rates: HashMap<String, f64>,
    pub time_last_update_utc: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DetailedErrorResponse {
    pub error: String,
    pub code: String, 
    pub request_id: String,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_currencies: Option<Vec<AvailableCurrency>>,
}