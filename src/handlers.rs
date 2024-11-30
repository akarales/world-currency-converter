//! src/handlers.rs
//! Handles simple currency conversion requests

use crate::{
    models::{ConversionRequest, SimpleConversionResponse, Validate, CurrencyInfo, DetailedErrorResponse},
    errors::ServiceError,
    clients::{HttpClient, CountryClient, ExchangeRateClient},
    data::GLOBAL_DATA,
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

fn format_country_name(name: &str) -> String {
    name.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let mut word = first.to_uppercase().collect::<String>();
                    word.extend(chars.map(|c| c.to_lowercase().next().unwrap_or(c)));
                    word
                }
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
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
    debug!("Falling back to first available currency for {}", country_name);
    currencies
        .iter()
        .next()
        .map(|(code, info)| (code.as_str(), info))
        .ok_or_else(|| ServiceError::InvalidCurrency(
            format!("No valid currency found for {}", country_name)
        ))
}

fn round_to_cents(amount: f64) -> f64 {
    (amount * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;
    use actix_web::{test, web::Data};
    use serde_json::json;

    static INIT: Once = Once::new();

    fn setup() {
        INIT.call_once(|| {
            env::set_var("RUST_LOG", "debug");
            env_logger::init();
        });
    }

    #[actix_web::test]
    async fn test_simple_conversion() {
        setup();
        
        let app = test::init_service(
            actix_web::App::new()
                .app_data(Data::new(Client::new()))
                .service(web::resource("/currency")
                    .route(web::post().to(convert_currency)))
        ).await;

        env::set_var("EXCHANGE_RATE_API_KEY", "test_key");

        let req = test::TestRequest::post()
            .uri("/currency")
            .insert_header(("content-type", "application/json"))
            .set_payload(json!({
                "from": "United States",
                "to": "France",
                "amount": 100.0,
                "preferred_currency": null
            }).to_string())
            .to_request();

        let resp = test::call_service(&app, req).await;
        let status = resp.status();
        
        assert!(
            status.is_success(),
            "Response status: {}, expected success",
            status
        );

        let body: SimpleConversionResponse = test::read_body_json(resp).await;
        
        assert_eq!(body.from, "USD");
        assert_eq!(body.to, "EUR");
        assert!(body.amount > 0.0);
    }

    #[actix_web::test]
    async fn test_invalid_country() {
        setup();
        
        let app = test::init_service(
            actix_web::App::new()
                .app_data(Data::new(Client::new()))
                .service(web::resource("/currency")
                    .route(web::post().to(convert_currency)))
        ).await;

        let req = test::TestRequest::post()
            .uri("/currency")
            .insert_header(("content-type", "application/json"))
            .set_payload(json!({
                "from": "Narnia",
                "to": "France",
                "amount": 100.0,
                "preferred_currency": null
            }).to_string())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 404);

        let body: DetailedErrorResponse = test::read_body_json(resp).await;
        assert!(body.error.contains("Country not found"));
    }

    #[actix_web::test]
    async fn test_service_errors() {
        setup();
        
        let app = test::init_service(
            actix_web::App::new()
                .app_data(Data::new(Client::new()))
                .service(web::resource("/currency")
                    .route(web::post().to(convert_currency)))
        ).await;

        env::remove_var("EXCHANGE_RATE_API_KEY");

        let req = test::TestRequest::post()
            .uri("/currency")
            .insert_header(("content-type", "application/json"))
            .set_payload(json!({
                "from": "United States",
                "to": "France",
                "amount": 100.0,
                "preferred_currency": null
            }).to_string())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 500);

        let body: DetailedErrorResponse = test::read_body_json(resp).await;
        assert!(body.error.contains("Service configuration error"));
        assert_eq!(body.details.as_deref(), Some("API key not configured"));
    }

    #[tokio::test]
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
        let result = select_currency(&currencies, Some("USD"), Some("EUR"), "Test")
            .unwrap();
        assert_eq!(result.0, "USD");

        // Test invalid preferred currency
        let result = select_currency(&currencies, Some("GBP"), Some("EUR"), "Test");
        assert!(result.is_err());

        // Test primary currency fallback
        let result = select_currency(&currencies, None, Some("EUR"), "Test")
            .unwrap();
        assert_eq!(result.0, "EUR");

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

        let result = select_currency(&currencies, None, None, "Test")
            .unwrap();
        assert_eq!(result.0, "USD");
    }

    #[tokio::test]
    async fn test_format_country_name() {
        let test_cases = vec![
            ("united states", "United States"),
            ("JAPAN", "Japan"),
            ("new   zealand", "New Zealand"),
            ("great  britain", "Great Britain"),
            ("   france   ", "France"),
        ];

        for (input, expected) in test_cases {
            assert_eq!(format_country_name(input), expected);
        }
    }

    #[tokio::test]
    async fn test_round_to_cents() {
        let test_cases = vec![
            (10.456, 10.46),
            (10.454, 10.45),
            (0.0, 0.0),
            (99.999, 100.0),
            (-10.456, -10.46),
        ];

        for (input, expected) in test_cases {
            assert_eq!(round_to_cents(input), expected);
        }
    }

    #[actix_web::test]
    async fn test_validation_errors() {
        setup();
        
        let app = test::init_service(
            actix_web::App::new()
                .app_data(Data::new(Client::new()))
                .service(web::resource("/currency")
                    .route(web::post().to(convert_currency)))
        ).await;

        let test_cases = vec![
            (json!({
                "from": "", 
                "to": "France", 
                "amount": 100.0,
                "preferred_currency": null
            }), "Source country cannot be empty"),
            (json!({
                "from": "USA", 
                "to": "", 
                "amount": 100.0,
                "preferred_currency": null
            }), "Destination country cannot be empty"),
            (json!({
                "from": "USA", 
                "to": "France", 
                "amount": 0.0,
                "preferred_currency": null
            }), "Amount must be greater than 0"),
        ];

        for (payload, expected_error) in test_cases {
            let req = test::TestRequest::post()
                .uri("/currency")
                .insert_header(("content-type", "application/json"))
                .set_payload(payload.to_string())
                .to_request();

            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status().as_u16(), 400);

            let body: DetailedErrorResponse = test::read_body_json(resp).await;
            assert!(body.error.contains(expected_error));
        }
    }

    #[actix_web::test]
    async fn test_same_currency_conversion() {
        setup();
        
        let app = test::init_service(
            actix_web::App::new()
                .app_data(Data::new(Client::new()))
                .service(web::resource("/currency")
                    .route(web::post().to(convert_currency)))
        ).await;

        env::set_var("EXCHANGE_RATE_API_KEY", "test_key");

        let req = test::TestRequest::post()
            .uri("/currency")
            .insert_header(("content-type", "application/json"))
            .set_payload(json!({
                "from": "France",
                "to": "France",
                "amount": 100.0,
                "preferred_currency": null
            }).to_string())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body: SimpleConversionResponse = test::read_body_json(resp).await;
        assert_eq!(body.from, body.to);
        assert_eq!(body.amount, 100.0);
    }
}