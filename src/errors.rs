use thiserror::Error;
use chrono::{DateTime, Utc};
use serde::Serialize;
use actix_web::{HttpResponse, ResponseError};
use crate::models::AvailableCurrency;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Country not found: {}", .0)]
    CountryNotFound(String),
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("External API error: {}", .0)]
    ExternalApiError(String),
    
    #[error("Cache error: {}", .0)]
    CacheError(String),
    
    #[error("Configuration error: {}", .0)]
    ConfigError(String),
    
    #[error("Invalid currency: {}", .0)]
    InvalidCurrency(String),
    
    #[error("Service unavailable: {}", .0)]
    ServiceUnavailable(String),

    #[error("Registry error: {}", .0)]
    RegistryError(String),

    #[error("Initialization error: {}", .0)]
    InitializationError(String),

    #[error("REST Countries API error: {}", .0)]
    RestCountriesError(String),

    #[error("Multiple currencies available: {}", .message)]
    MultipleCurrenciesError {
        message: String,
        available_currencies: Vec<AvailableCurrency>,
    },

    #[error("Currency not supported: {}", .0)]
    CurrencyNotSupported(String),
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
    pub request_id: String,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_currencies: Option<Vec<AvailableCurrency>>,
}

impl ErrorResponse {
    pub fn new(error: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            code: code.into(),
            request_id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            details: None,
            available_currencies: None,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    pub fn with_currencies(mut self, currencies: Vec<AvailableCurrency>) -> Self {
        self.available_currencies = Some(currencies);
        self
    }
}

impl ResponseError for ServiceError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ServiceError::RateLimitExceeded => {
                let error_response = ErrorResponse::new(
                    self.to_string(),
                    "RATE_LIMIT_EXCEEDED",
                );
                HttpResponse::TooManyRequests().json(error_response)
            }
            ServiceError::CountryNotFound(country) => {
                let error_response = ErrorResponse::new(
                    format!("Country not found: {}", country),
                    "COUNTRY_NOT_FOUND",
                );
                HttpResponse::NotFound().json(error_response)
            }
            ServiceError::InvalidCurrency(msg) => {
                let error_response = ErrorResponse::new(
                    format!("Invalid currency: {}", msg),
                    "INVALID_CURRENCY",
                );
                HttpResponse::BadRequest().json(error_response)
            }
            ServiceError::MultipleCurrenciesError { message, available_currencies } => {
                let error_response = ErrorResponse::new(
                    format!("Multiple currencies available: {}", message),
                    "MULTIPLE_CURRENCIES",
                ).with_currencies(available_currencies.clone());
                HttpResponse::BadRequest().json(error_response)
            }
            ServiceError::CurrencyNotSupported(currency) => {
                let error_response = ErrorResponse::new(
                    format!("Currency not supported: {}", currency),
                    "CURRENCY_NOT_SUPPORTED",
                );
                HttpResponse::BadRequest().json(error_response)
            }
            ServiceError::RestCountriesError(msg) => {
                let error_response = ErrorResponse::new(
                    format!("REST Countries API error: {}", msg),
                    "REST_COUNTRIES_ERROR",
                );
                HttpResponse::ServiceUnavailable().json(error_response)
            }
            ServiceError::ExternalApiError(msg) | ServiceError::ServiceUnavailable(msg) => {
                let error_response = ErrorResponse::new(
                    "Service temporarily unavailable",
                    "SERVICE_UNAVAILABLE",
                ).with_details(msg);
                HttpResponse::ServiceUnavailable().json(error_response)
            }
            ServiceError::ConfigError(msg) | ServiceError::InitializationError(msg) => {
                let error_response = ErrorResponse::new(
                    "Service configuration error",
                    "CONFIG_ERROR",
                ).with_details(msg);
                HttpResponse::InternalServerError().json(error_response)
            }
            _ => {
                let error_response = ErrorResponse::new(
                    "Internal server error",
                    "INTERNAL_ERROR",
                ).with_details(self.to_string());
                HttpResponse::InternalServerError().json(error_response)
            }
        }
    }
}

impl From<reqwest::Error> for ServiceError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            ServiceError::ServiceUnavailable("Request timeout".to_string())
        } else if err.is_connect() {
            ServiceError::ServiceUnavailable("Connection error".to_string())
        } else {
            ServiceError::ExternalApiError(err.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_response_creation() {
        let response = ErrorResponse::new("Test error", "TEST_ERROR")
            .with_details("Additional details")
            .with_currencies(vec![
                AvailableCurrency {
                    code: "USD".to_string(),
                    name: "US Dollar".to_string(),
                    symbol: "$".to_string(),
                    is_primary: true,
                }
            ]);
        
        assert!(!response.request_id.is_empty());
        assert_eq!(response.code, "TEST_ERROR");
        assert_eq!(response.details.unwrap(), "Additional details");
        assert!(response.available_currencies.is_some());
        assert_eq!(response.available_currencies.unwrap()[0].code, "USD");
    }

    #[test]
    fn test_service_error_conversion() {
        // Test CountryNotFound error
        let error = ServiceError::CountryNotFound("Test".to_string());
        let response = error.error_response();
        assert_eq!(response.status(), 404);

        // Test MultipleCurrenciesError
        let error = ServiceError::MultipleCurrenciesError {
            message: "Multiple currencies found".to_string(),
            available_currencies: vec![
                AvailableCurrency {
                    code: "USD".to_string(),
                    name: "US Dollar".to_string(),
                    symbol: "$".to_string(),
                    is_primary: true,
                }
            ],
        };
        let response = error.error_response();
        assert_eq!(response.status(), 400);

        // Test REST Countries API error
        let error = ServiceError::RestCountriesError("API error".to_string());
        let response = error.error_response();
        assert_eq!(response.status(), 503);
    }

    #[tokio::test]
    async fn test_reqwest_error_conversion() {
        // Create a timeout error
        let err = reqwest::get("http://localhost:1")
            .await
            .unwrap_err();
        let service_error = ServiceError::from(err);
        
        match service_error {
            ServiceError::ServiceUnavailable(_) => (),
            _ => panic!("Expected ServiceUnavailable error"),
        }
    }
}