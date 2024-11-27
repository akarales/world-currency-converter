use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct DetailedErrorResponse {
    pub error: String,
    pub request_id: String,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_currencies: Option<Vec<AvailableCurrency>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExchangeRateResponse {
    pub result: String,
    pub conversion_rates: HashMap<String, f64>,
    pub time_last_update_utc: Option<String>,
}

// New validation traits
pub trait Validate {
    fn validate(&self) -> Result<(), crate::errors::ServiceError>;
}

impl Validate for ConversionRequest {
    fn validate(&self) -> Result<(), crate::errors::ServiceError> {
        if self.amount <= 0.0 {
            return Err(crate::errors::ServiceError::InvalidCurrency(
                "Amount must be greater than 0".to_string(),
            ));
        }
        if self.from.trim().is_empty() || self.to.trim().is_empty() {
            return Err(crate::errors::ServiceError::InvalidCurrency(
                "Country names cannot be empty".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversion_request_validation() {
        let valid_request = ConversionRequest {
            from: "USA".to_string(),
            to: "France".to_string(),
            amount: 100.0,
            preferred_currency: None,
        };
        assert!(valid_request.validate().is_ok());

        let invalid_amount = ConversionRequest {
            from: "USA".to_string(),
            to: "France".to_string(),
            amount: 0.0,
            preferred_currency: None,
        };
        assert!(invalid_amount.validate().is_err());

        let invalid_country = ConversionRequest {
            from: "".to_string(),
            to: "France".to_string(),
            amount: 100.0,
            preferred_currency: None,
        };
        assert!(invalid_country.validate().is_err());
    }

    #[test]
    fn test_response_metadata_serialization() {
        let metadata = ResponseMetadata {
            source: "test".to_string(),
            response_time_ms: 100,
            multiple_currencies_available: false,
            rate_limit_remaining: Some(100),
            cache_hit: Some(true),
        };
        let serialized = serde_json::to_string(&metadata).unwrap();
        assert!(serialized.contains("rate_limit_remaining"));
        assert!(serialized.contains("cache_hit"));
    }

    #[test]
    fn test_available_currency_equality() {
        let currency1 = AvailableCurrency {
            code: "USD".to_string(),
            name: "US Dollar".to_string(),
            symbol: "$".to_string(),
            is_primary: true,
        };
        let currency2 = AvailableCurrency {
            code: "USD".to_string(),
            name: "US Dollar".to_string(),
            symbol: "$".to_string(),
            is_primary: true,
        };
        assert_eq!(currency1, currency2);
    }
}