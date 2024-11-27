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
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
    pub request_id: String,
    pub timestamp: DateTime<Utc>,
}

impl ErrorResponse {
    pub fn new(error: String, code: String) -> Self {
        Self {
            error,
            code,
            request_id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
        }
    }
}

impl ResponseError for ServiceError {
    fn error_response(&self) -> HttpResponse {
        let response = match self {
            ServiceError::RateLimitExceeded => {
                let error_response = ErrorResponse::new(
                    self.to_string(),
                    "RATE_LIMIT_EXCEEDED".to_string(),
                );
                HttpResponse::TooManyRequests().json(error_response)
            }
            ServiceError::CountryNotFound(_) => {
                let error_response = ErrorResponse::new(
                    self.to_string(),
                    "COUNTRY_NOT_FOUND".to_string(),
                );
                HttpResponse::NotFound().json(error_response)
            }
            ServiceError::InvalidCurrency(_) => {
                let error_response = ErrorResponse::new(
                    self.to_string(),
                    "INVALID_CURRENCY".to_string(),
                );
                HttpResponse::BadRequest().json(error_response)
            }
            ServiceError::ExternalApiError(_) | ServiceError::ServiceUnavailable(_) => {
                let error_response = ErrorResponse::new(
                    "Service temporarily unavailable".to_string(),
                    "SERVICE_UNAVAILABLE".to_string(),
                );
                HttpResponse::ServiceUnavailable().json(error_response)
            }
            _ => {
                let error_response = ErrorResponse::new(
                    "Internal server error".to_string(),
                    "INTERNAL_ERROR".to_string(),
                );
                HttpResponse::InternalServerError().json(error_response)
            }
        };
        response
    }
}

impl From<reqwest::Error> for ServiceError {
    fn from(err: reqwest::Error) -> Self {
        ServiceError::ExternalApiError(err.to_string())
    }
}