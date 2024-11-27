use thiserror::Error;
use chrono::{DateTime, Utc};
use serde::Serialize;
use actix_web::{HttpResponse, ResponseError};

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Country not found: {0}")]
    CountryNotFound(String),
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("External API error: {0}")]
    ExternalApiError(String),
    
    #[error("Cache error: {0}")]
    CacheError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Invalid currency: {0}")]
    InvalidCurrency(String),
    
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Registry error: {0}")]
    RegistryError(String),

    #[error("Initialization error: {0}")]
    InitializationError(String),
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
    pub request_id: String,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl ErrorResponse {
    pub fn new(error: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            code: code.into(),
            request_id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
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
            ServiceError::CountryNotFound(_) => {
                let error_response = ErrorResponse::new(
                    self.to_string(),
                    "COUNTRY_NOT_FOUND",
                );
                HttpResponse::NotFound().json(error_response)
            }
            ServiceError::InvalidCurrency(_) => {
                let error_response = ErrorResponse::new(
                    self.to_string(),
                    "INVALID_CURRENCY",
                );
                HttpResponse::BadRequest().json(error_response)
            }
            ServiceError::ExternalApiError(_) | ServiceError::ServiceUnavailable(_) => {
                let error_response = ErrorResponse::new(
                    "Service temporarily unavailable",
                    "SERVICE_UNAVAILABLE",
                ).with_details(self.to_string());
                HttpResponse::ServiceUnavailable().json(error_response)
            }
            ServiceError::ConfigError(_) | ServiceError::InitializationError(_) => {
                let error_response = ErrorResponse::new(
                    "Service configuration error",
                    "CONFIG_ERROR",
                ).with_details(self.to_string());
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
        ServiceError::ExternalApiError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_response_creation() {
        let response = ErrorResponse::new("Test error", "TEST_ERROR")
            .with_details("Additional details");
        
        assert!(!response.request_id.is_empty());
        assert_eq!(response.code, "TEST_ERROR");
        assert_eq!(response.details.unwrap(), "Additional details");
    }

    #[test]
    fn test_service_error_conversion() {
        let error = ServiceError::CountryNotFound("Test".to_string());
        let response = error.error_response();
        assert_eq!(response.status(), 404);
    }
}