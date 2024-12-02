use crate::{
    models::{ConversionRequest, SimpleConversionResponse, Validate, CurrencyInfo, DetailedErrorResponse},
    errors::ServiceError,
    clients::{HttpClient, CountryClient, ExchangeRateClient},
    data::GLOBAL_DATA,
    format_country_name,
    round_to_cents,
};
use actix_web::{web, HttpResponse, Error};
use log::{debug, info, error};
use reqwest::Client;
use std::{collections::HashMap, env};
use uuid::Uuid;
use chrono::Utc;

pub async fn convert_currency(
    request: web::Json<ConversionRequest>,
    client: web::Data<Client>,
) -> Result<HttpResponse, Error> {
    debug!("Processing simple conversion request: {:?}", request);

    // Validate request
    if let Err(e) = request.validate() {
        debug!("Validation error: {}", e);
        return Ok(HttpResponse::BadRequest().json(DetailedErrorResponse {
            error: format!("Invalid currency: {}", e),
            code: "INVALID_CURRENCY".to_string(),
            request_id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            details: None,
            available_currencies: None,
        }));
    }

    let api_key = match env::var("EXCHANGE_RATE_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            debug!("API key missing");
            return Ok(HttpResponse::InternalServerError().json(DetailedErrorResponse {
                error: "Service configuration error".to_string(),
                code: "CONFIG_ERROR".to_string(),
                request_id: Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                details: Some("API key not configured".to_string()),
                available_currencies: None,
            }));
        }
    };

    let http_client = HttpClient::new(client.get_ref().clone(), api_key);
    let from_country = format_country_name(&request.from);
    let to_country = format_country_name(&request.to);

    // Get source country info
    let from_country_info = match http_client.get_country_info(&from_country).await {
        Ok(info) => {
            debug!("Got country info for {}: {:?}", from_country, info);
            info
        },
        Err(e) => {
            debug!("Country not found: {} - {}", from_country, e);
            return Ok(HttpResponse::NotFound().json(DetailedErrorResponse {
                error: format!("Country not found: {}", from_country),
                code: "COUNTRY_NOT_FOUND".to_string(),
                request_id: Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                details: None,
                available_currencies: None,
            }));
        }
    };

    let to_country_info = match http_client.get_country_info(&to_country).await {
        Ok(info) => {
            debug!("Got country info for {}: {:?}", to_country, info);
            info
        },
        Err(e) => {
            debug!("Country not found: {} - {}", to_country, e);
            return Ok(HttpResponse::NotFound().json(DetailedErrorResponse {
                error: format!("Country not found: {}", to_country),
                code: "COUNTRY_NOT_FOUND".to_string(),
                request_id: Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                details: None,
                available_currencies: None,
            }));
        }
    };

    // Get currencies for both countries
    let from_currencies = match &from_country_info.currencies {
        Some(currencies) => currencies,
        None => {
            return Ok(HttpResponse::BadRequest().json(DetailedErrorResponse {
                error: format!("No currencies found for {}", from_country),
                code: "INVALID_CURRENCY".to_string(),
                request_id: Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                details: None,
                available_currencies: None,
            }));
        }
    };

    let to_currencies = match &to_country_info.currencies {
        Some(currencies) => currencies,
        None => {
            return Ok(HttpResponse::BadRequest().json(DetailedErrorResponse {
                error: format!("No currencies found for {}", to_country),
                code: "INVALID_CURRENCY".to_string(),
                request_id: Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                details: None,
                available_currencies: None,
            }));
        }
    };

    // Get primary currencies
    let from_primary = GLOBAL_DATA.get().unwrap().get_primary_currency(&from_country).await;
    let to_primary = GLOBAL_DATA.get().unwrap().get_primary_currency(&to_country).await;

    debug!("Primary currencies - From: {:?}, To: {:?}", from_primary, to_primary);

    // Select currencies
    let (from_currency_code, _) = select_currency(
        from_currencies,
        request.preferred_currency.as_deref(),
        from_primary.as_deref(),
        &from_country,
    ).map_err(|e| {
        debug!("Currency selection error: {}", e);
        Error::from(e)
    })?;

    let (to_currency_code, _) = select_currency(
        to_currencies,
        request.preferred_currency.as_deref(),
        to_primary.as_deref(),
        &to_country,
    ).map_err(|e| {
        debug!("Currency selection error: {}", e);
        Error::from(e)
    })?;

    debug!("Selected currencies - From: {}, To: {}", from_currency_code, to_currency_code);

    // Handle same currency case
    if from_currency_code == to_currency_code {
        debug!("Same currency conversion, returning original amount");
        return Ok(HttpResponse::Ok().json(SimpleConversionResponse {
            from: from_currency_code.to_string(),
            to: to_currency_code.to_string(),
            amount: round_to_cents(request.amount),
        }));
    }

    // Get conversion rate and perform conversion
    let response = match http_client.get_exchange_rate(from_currency_code).await {
        Ok(r) => {
            debug!("Got exchange rate response: {:?}", r);
            r
        },
        Err(e) => {
            error!("Exchange rate service error: {}", e);
            return Ok(HttpResponse::ServiceUnavailable().json(DetailedErrorResponse {
                error: format!("Service temporarily unavailable"),
                code: "SERVICE_UNAVAILABLE".to_string(),
                request_id: Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                details: Some(format!("Failed to get exchange rate: {}", e)),
                available_currencies: None,
            }));
        }
    };

    let rate = match response.conversion_rates.get(to_currency_code) {
        Some(r) => r,
        None => {
            return Ok(HttpResponse::BadRequest().json(DetailedErrorResponse {
                error: format!("Exchange rate not found for {}->{}", 
                    from_currency_code, 
                    to_currency_code
                ),
                code: "INVALID_CURRENCY".to_string(),
                request_id: Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                details: None,
                available_currencies: None,
            }));
        }
    };

    let converted_amount = round_to_cents(request.amount * rate);
    
    info!(
        "Conversion successful: {} {} -> {} {}",
        request.amount, from_currency_code, converted_amount, to_currency_code
    );

    Ok(HttpResponse::Ok().json(SimpleConversionResponse {
        from: from_currency_code.to_string(),
        to: to_currency_code.to_string(),
        amount: converted_amount,
    }))
}

fn select_currency<'a>(
    currencies: &'a HashMap<String, CurrencyInfo>,
    preferred: Option<&'a str>,
    primary: Option<&'a str>,
    country_name: &str,
) -> Result<(&'a str, &'a CurrencyInfo), ServiceError> {
    debug!("Selecting currency for {} - Preferred: {:?}, Primary: {:?}, Available: {:?}",
        country_name, preferred, primary, currencies.keys());

    // Try preferred currency first
    if let Some(pref) = preferred {
        if let Some(info) = currencies.get(pref) {
            debug!("Using preferred currency {} for {}", pref, country_name);
            return Ok((pref, info));
        } else {
            debug!("Preferred currency {} not available for {}", pref, country_name);
            return Err(ServiceError::InvalidCurrency(
                format!("Preferred currency {} not available for {}. Available currencies: {}", 
                    pref,
                    country_name,
                    currencies.keys().cloned().collect::<Vec<_>>().join(", ")
                )
            ));
        }
    }

    // Try primary currency
    if let Some(primary_code) = primary {
        if let Some(info) = currencies.get(primary_code) {
            debug!("Using primary currency {} for {}", primary_code, country_name);
            return Ok((primary_code, info));
        }
    }

    // Try USD or EUR
    for code in ["USD", "EUR"] {
        if let Some(info) = currencies.get(code) {
            debug!("Using standard currency {} for {}", code, country_name);
            return Ok((code, info));
        }
    }

    // Fall back to first available
    currencies
        .iter()
        .next()
        .map(|(code, info)| (code.as_str(), info))
        .ok_or_else(|| ServiceError::InvalidCurrency(
            format!("No valid currency found for {}", country_name)
        ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App, dev::Service, dev::ServiceRequest, http::header};
    use crate::test_utils::mocks::MockHttpClient;
    use crate::test_utils::init_test_env;
    use serde_json::json;
    use std::sync::Once;

    static INIT: Once = Once::new();

    impl Default for MockHttpClient {
        fn default() -> Self {
            Self {}
        }
    }

    async fn setup_test_app() -> impl Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = actix_web::Error> {
        INIT.call_once(|| {
            std::env::set_var("RUST_LOG", "debug");
            env_logger::try_init().ok();
        });
    
        // Initialize test environment (no await)
        init_test_env();
        
        test::init_service(
            App::new()
                .app_data(web::Data::new(MockHttpClient::default()))
                .service(
                    web::resource("/currency")
                        .route(web::post().to(convert_currency))
                )
        ).await
    }

    fn create_test_request(payload: serde_json::Value) -> ServiceRequest {
        test::TestRequest::post()
            .uri("/currency")
            .insert_header((header::CONTENT_TYPE, "application/json"))
            .set_payload(payload.to_string())
            .to_srv_request()
    }

    #[actix_rt::test]
    async fn test_simple_conversion() {
        let app = setup_test_app().await;
        
        std::env::set_var("EXCHANGE_RATE_API_KEY", "test_key");

        let req = create_test_request(json!({
            "from": "United States",
            "to": "France",
            "amount": 100.0,
            "preferred_currency": null
        }));

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success(), 
            "Response status: {}, expected success", 
            resp.status()
        );

        let body: SimpleConversionResponse = test::read_body_json(resp).await;
        assert_eq!(body.from, "USD");
        assert_eq!(body.to, "EUR");
        assert!(body.amount > 0.0);
    }

    #[actix_rt::test]
    async fn test_invalid_country() {
        let app = setup_test_app().await;

        let req = create_test_request(json!({
            "from": "Narnia",
            "to": "France",
            "amount": 100.0,
            "preferred_currency": null
        }));

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 404);

        let body: DetailedErrorResponse = test::read_body_json(resp).await;
        assert!(body.error.contains("Country not found"));
        assert_eq!(body.code, "COUNTRY_NOT_FOUND");
    }

    #[actix_rt::test]
    async fn test_service_errors() {
        let app = setup_test_app().await;
        
        let original_key = std::env::var("EXCHANGE_RATE_API_KEY").ok();
        std::env::remove_var("EXCHANGE_RATE_API_KEY");

        let req = create_test_request(json!({
            "from": "United States",
            "to": "France",
            "amount": 100.0,
            "preferred_currency": null
        }));

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 500);

        let body: DetailedErrorResponse = test::read_body_json(resp).await;
        assert!(body.error.contains("Service configuration error"));
        assert_eq!(body.code, "CONFIG_ERROR");
        assert_eq!(body.details.as_deref(), Some("API key not configured"));

        if let Some(key) = original_key {
            std::env::set_var("EXCHANGE_RATE_API_KEY", key);
        }
    }

    #[actix_rt::test]
    async fn test_select_currency() {
        let mut currencies = HashMap::new();
        currencies.insert(
            "USD".to_string(), 
            CurrencyInfo {
                name: "US Dollar".to_string(),
                symbol: "$".to_string(),
            }
        );
        currencies.insert(
            "EUR".to_string(),
            CurrencyInfo {
                name: "Euro".to_string(),
                symbol: "€".to_string(),
            }
        );

        // Test preferred currency
        let (code, info) = select_currency(&currencies, Some("USD"), Some("EUR"), "Test")
            .expect("Currency selection failed");
        assert_eq!(code, "USD");
        assert_eq!(info.symbol, "$");

        // Test invalid preferred currency
        let result = select_currency(&currencies, Some("GBP"), Some("EUR"), "Test");
        assert!(matches!(result, Err(ServiceError::InvalidCurrency(_))));

        // Test primary currency fallback
        let (code, info) = select_currency(&currencies, None, Some("EUR"), "Test")
            .expect("Currency selection failed");
        assert_eq!(code, "EUR");
        assert_eq!(info.symbol, "€");

        // Test USD/EUR priority
        let mut currencies = HashMap::new();
        currencies.insert(
            "GBP".to_string(),
            CurrencyInfo {
                name: "British Pound".to_string(),
                symbol: "£".to_string(),
            }
        );
        currencies.insert(
            "USD".to_string(),
            CurrencyInfo {
                name: "US Dollar".to_string(),
                symbol: "$".to_string(),
            }
        );

        let (code, info) = select_currency(&currencies, None, None, "Test")
            .expect("Currency selection failed");
        assert_eq!(code, "USD");
        assert_eq!(info.symbol, "$");
    }

    #[actix_rt::test]
    async fn test_validation_errors() {
        let app = setup_test_app().await;

        let test_cases = vec![
            (json!({
                "from": "", 
                "to": "France", 
                "amount": 100.0,
                "preferred_currency": null
            }), "Source country cannot be empty", 400),
            (json!({
                "from": "USA", 
                "to": "", 
                "amount": 100.0,
                "preferred_currency": null
            }), "Destination country cannot be empty", 400),
            (json!({
                "from": "USA", 
                "to": "France", 
                "amount": 0.0,
                "preferred_currency": null
            }), "Amount must be greater than 0", 400),
            (json!({
                "from": "USA", 
                "to": "France", 
                "amount": -1.0,
                "preferred_currency": null
            }), "Amount must be greater than 0", 400),
        ];

        for (payload, expected_error, expected_status) in test_cases {
            let req = create_test_request(payload);

            let resp = test::call_service(&app, req).await;
            assert_eq!(
                resp.status().as_u16(), 
                expected_status,
                "Expected status {} for error: {}", 
                expected_status, 
                expected_error
            );

            let body: DetailedErrorResponse = test::read_body_json(resp).await;
            assert!(
                body.error.contains(expected_error),
                "Expected error '{}' but got '{}'",
                expected_error,
                body.error
            );
            assert_eq!(body.code, "INVALID_CURRENCY");
        }
    }

    #[actix_rt::test]
    async fn test_same_currency_conversion() {
        let app = setup_test_app().await;

        std::env::set_var("EXCHANGE_RATE_API_KEY", "test_key");

        let req = create_test_request(json!({
            "from": "France",
            "to": "France",
            "amount": 100.0,
            "preferred_currency": null
        }));

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body: SimpleConversionResponse = test::read_body_json(resp).await;
        assert_eq!(body.from, "EUR");
        assert_eq!(body.to, "EUR");
        assert_eq!(body.amount, 100.0);
    }

    #[actix_rt::test]
    async fn test_multi_currency_conversions() {
        let app = setup_test_app().await;

        std::env::set_var("EXCHANGE_RATE_API_KEY", "test_key");

        let req = create_test_request(json!({
            "from": "Zimbabwe",
            "to": "France",
            "amount": 100.0,
            "preferred_currency": "USD"
        }));

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body: SimpleConversionResponse = test::read_body_json(resp).await;
        assert_eq!(body.from, "USD");
        assert_eq!(body.to, "EUR");
        assert!(body.amount > 0.0);
    }
}