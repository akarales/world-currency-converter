use crate::{
    models::*,
    currency_service::CurrencyService
};
use actix_web::{web, HttpResponse, http::header::ContentType};
use log::{debug, error, info};
use chrono::Utc;
use uuid::Uuid;
use std::env;

pub async fn convert_currency(
    data: web::Json<ConversionRequest>,
    client: web::Data<reqwest::Client>,
) -> Result<HttpResponse, actix_web::Error> {
    debug!("Processing v1 conversion request: {:?}", data);

    // Check for API key first - before any other operations
    if env::var("EXCHANGE_RATE_API_KEY").is_err() {
        error!("Exchange rate API key not found");
        return Ok(HttpResponse::ServiceUnavailable()
            .content_type(ContentType::json())
            .json(DetailedErrorResponse {
                error: "Exchange rate API key not found".to_string(),
                request_id: Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                available_currencies: None,
            }));
    }

    let service = CurrencyService::new(client.get_ref().clone());

    match service.convert_currency(&data).await {
        Ok(response) => {
            info!(
                "Conversion successful: {} {} -> {} {} (rate: {})",
                response.data.from.amount,
                response.data.from.currency_code,
                response.data.to.amount,
                response.data.to.currency_code,
                response.data.exchange_rate
            );
            Ok(HttpResponse::Ok()
                .content_type(ContentType::json())
                .json(response))
        }
        Err(e) => {
            error!("Conversion failed: {}", e);
            Ok(HttpResponse::BadRequest()
                .content_type(ContentType::json())
                .json(DetailedErrorResponse {
                    error: e.to_string(),
                    request_id: Uuid::new_v4().to_string(),
                    timestamp: Utc::now(),
                    available_currencies: None,
                }))
        }
    }
}