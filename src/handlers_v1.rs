//! src/handlers_v1.rs
//! Handles detailed currency conversion requests with extra metadata and validation

use crate::{
    models::{
        ConversionRequest, DetailedConversionResponse, ConversionData, CurrencyDetails, 
        ResponseMetadata, AvailableCurrency, CurrencyInfo, Validate,
    },
    errors::ServiceError,
    clients::{CountryClient, ExchangeRateClient},
    data::GLOBAL_DATA,
};
use actix_web::{web, Error};
use log::{debug, info, error};
use reqwest::Client;
use std::{collections::HashMap, env};
use chrono::Utc;
use uuid::Uuid;
use crate::clients::HttpClient;

fn get_available_currencies(
    from_currencies: &HashMap<String, CurrencyInfo>,
    to_currencies: &HashMap<String, CurrencyInfo>,
    from_primary: &Option<String>,
    to_primary: &Option<String>,
) -> Vec<AvailableCurrency> {
    let mut currencies = Vec::new();
    debug!("Building available currencies - From: {:?}, To: {:?}", from_currencies, to_currencies);
    
    // Add source country currencies
    for (code, info) in from_currencies {
        let is_primary = from_primary.as_ref().map_or(false, |p| p == code);
        currencies.push(AvailableCurrency {
            code: code.clone(),
            name: info.name.clone(),
            symbol: info.symbol.clone(),
            is_primary,
        });
    }

    // Add destination country currencies if different
    for (code, info) in to_currencies {
        if !currencies.iter().any(|c| &c.code == code) {
            let is_primary = to_primary.as_ref().map_or(false, |p| p == code);
            currencies.push(AvailableCurrency {
                code: code.clone(),
                name: info.name.clone(),
                symbol: info.symbol.clone(),
                is_primary,
            });
        }
    }

    debug!("Built available currencies: {:?}", currencies);
    currencies
}

pub async fn convert_currency(
    request: web::Json<ConversionRequest>,
    client: web::Data<Client>,
) -> Result<web::Json<DetailedConversionResponse>, Error> {
    let start_time = std::time::Instant::now();
    let request_id = Uuid::new_v4().to_string();
    
    debug!("Processing v1 conversion request: {:?}", request);

    // Validate request
    if let Err(e) = request.validate() {
        debug!("Validation error: {}", e);
        return Ok(web::Json(DetailedConversionResponse {
            request_id,
            timestamp: Utc::now(),
            data: ConversionData {
                from: CurrencyDetails {
                    country: request.from.clone(),
                    currency_code: "INVALID".to_string(),
                    currency_name: "Invalid Currency".to_string(),
                    currency_symbol: "".to_string(),
                    amount: request.amount,
                    is_primary: false,
                },
                to: CurrencyDetails {
                    country: request.to.clone(),
                    currency_code: "INVALID".to_string(),
                    currency_name: "Invalid Currency".to_string(),
                    currency_symbol: "".to_string(),
                    amount: 0.0,
                    is_primary: false,
                },
                exchange_rate: 0.0,
                last_updated: Utc::now(),
                available_currencies: None,
            },
            meta: ResponseMetadata {
                source: "exchangerate-api.com".to_string(),
                response_time_ms: start_time.elapsed().as_millis() as u64,
                multiple_currencies_available: false,
                cache_hit: Some(false),
                rate_limit_remaining: None,
            },
        }));
    }

    let api_key = match env::var("EXCHANGE_RATE_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            error!("API key missing");
            return Ok(web::Json(DetailedConversionResponse {
                request_id,
                timestamp: Utc::now(),
                data: ConversionData {
                    from: CurrencyDetails {
                        country: request.from.clone(),
                        currency_code: "INVALID".to_string(),
                        currency_name: "Invalid Currency".to_string(),
                        currency_symbol: "".to_string(),
                        amount: request.amount,
                        is_primary: false,
                    },
                    to: CurrencyDetails {
                        country: request.to.clone(),
                        currency_code: "INVALID".to_string(),
                        currency_name: "Invalid Currency".to_string(),
                        currency_symbol: "".to_string(),
                        amount: 0.0,
                        is_primary: false,
                    },
                    exchange_rate: 0.0,
                    last_updated: Utc::now(),
                    available_currencies: None,
                },
                meta: ResponseMetadata {
                    source: "exchangerate-api.com".to_string(),
                    response_time_ms: start_time.elapsed().as_millis() as u64,
                    multiple_currencies_available: false,
                    cache_hit: Some(false),
                    rate_limit_remaining: None,
                },
            }));
        }
    };

    let http_client = HttpClient::new(client.get_ref().clone(), api_key);
    let from_country = format_country_name(&request.from);
    let to_country = format_country_name(&request.to);

    // Get country information first
    let from_country_info = match http_client.get_country_info(&from_country).await {
        Ok(info) => {
            debug!("Got country info for {}: {:?}", from_country, info);
            info
        },
        Err(e) => {
            error!("Failed to get country info for {}: {}", from_country, e);
            return Ok(web::Json(DetailedConversionResponse {
                request_id,
                timestamp: Utc::now(),
                data: ConversionData {
                    from: CurrencyDetails {
                        country: from_country.clone(),
                        currency_code: "INVALID".to_string(),
                        currency_name: "Invalid Currency".to_string(),
                        currency_symbol: "".to_string(),
                        amount: request.amount,
                        is_primary: false,
                    },
                    to: CurrencyDetails {
                        country: to_country.clone(),
                        currency_code: "INVALID".to_string(),
                        currency_name: "Invalid Currency".to_string(),
                        currency_symbol: "".to_string(),
                        amount: 0.0,
                        is_primary: false,
                    },
                    exchange_rate: 0.0,
                    last_updated: Utc::now(),
                    available_currencies: None,
                },
                meta: ResponseMetadata {
                    source: "exchangerate-api.com".to_string(),
                    response_time_ms: start_time.elapsed().as_millis() as u64,
                    multiple_currencies_available: false,
                    cache_hit: Some(false),
                    rate_limit_remaining: None,
                },
            }));
        }
    };

    let to_country_info = match http_client.get_country_info(&to_country).await {
        Ok(info) => {
            debug!("Got country info for {}: {:?}", to_country, info);
            info
        },
        Err(e) => {
            error!("Failed to get country info for {}: {}", to_country, e);
            return Ok(web::Json(DetailedConversionResponse {
                request_id,
                timestamp: Utc::now(),
                data: ConversionData {
                    from: CurrencyDetails {
                        country: from_country.clone(),
                        currency_code: "INVALID".to_string(),
                        currency_name: "Invalid Currency".to_string(),
                        currency_symbol: "".to_string(),
                        amount: request.amount,
                        is_primary: false,
                    },
                    to: CurrencyDetails {
                        country: to_country.clone(),
                        currency_code: "INVALID".to_string(),
                        currency_name: "Invalid Currency".to_string(),
                        currency_symbol: "".to_string(),
                        amount: 0.0,
                        is_primary: false,
                    },
                    exchange_rate: 0.0,
                    last_updated: Utc::now(),
                    available_currencies: None,
                },
                meta: ResponseMetadata {
                    source: "exchangerate-api.com".to_string(),
                    response_time_ms: start_time.elapsed().as_millis() as u64,
                    multiple_currencies_available: false,
                    cache_hit: Some(false),
                    rate_limit_remaining: None,
                },
            }));
        }
    };

    // Get currencies for both countries
    let from_currencies = match &from_country_info.currencies {
        Some(currencies) => currencies,
        None => {
            error!("No currencies found for {}", from_country);
            return Ok(web::Json(DetailedConversionResponse {
                request_id,
                timestamp: Utc::now(),
                data: ConversionData {
                    from: CurrencyDetails {
                        country: from_country.clone(),
                        currency_code: "INVALID".to_string(),
                        currency_name: "Invalid Currency".to_string(),
                        currency_symbol: "".to_string(),
                        amount: request.amount,
                        is_primary: false,
                    },
                    to: CurrencyDetails {
                        country: to_country.clone(),
                        currency_code: "INVALID".to_string(),
                        currency_name: "Invalid Currency".to_string(),
                        currency_symbol: "".to_string(),
                        amount: 0.0,
                        is_primary: false,
                    },
                    exchange_rate: 0.0,
                    last_updated: Utc::now(),
                    available_currencies: None,
                },
                meta: ResponseMetadata {
                    source: "exchangerate-api.com".to_string(),
                    response_time_ms: start_time.elapsed().as_millis() as u64,
                    multiple_currencies_available: false,
                    cache_hit: Some(false),
                    rate_limit_remaining: None,
                },
            }));
        }
    };

    let to_currencies = match &to_country_info.currencies {
        Some(currencies) => currencies,
        None => {
            error!("No currencies found for {}", to_country);
            return Ok(web::Json(DetailedConversionResponse {
                request_id,
                timestamp: Utc::now(),
                data: ConversionData {
                    from: CurrencyDetails {
                        country: from_country.clone(),
                        currency_code: "INVALID".to_string(),
                        currency_name: "Invalid Currency".to_string(),
                        currency_symbol: "".to_string(),
                        amount: request.amount,
                        is_primary: false,
                    },
                    to: CurrencyDetails {
                        country: to_country.clone(),
                        currency_code: "INVALID".to_string(),
                        currency_name: "Invalid Currency".to_string(),
                        currency_symbol: "".to_string(),
                        amount: 0.0,
                        is_primary: false,
                    },
                    exchange_rate: 0.0,
                    last_updated: Utc::now(),
                    available_currencies: None,
                },
                meta: ResponseMetadata {
                    source: "exchangerate-api.com".to_string(),
                    response_time_ms: start_time.elapsed().as_millis() as u64,
                    multiple_currencies_available: false,
                    cache_hit: Some(false),
                    rate_limit_remaining: None,
                },
            }));
        }
    };

    // Check multi-currency status
    let from_multi = GLOBAL_DATA.get().unwrap().is_multi_currency(&from_country).await;
    let to_multi = GLOBAL_DATA.get().unwrap().is_multi_currency(&to_country).await;
    let multiple_currencies_available = from_multi || to_multi;

    debug!("Multi-currency status - From {}: {}, To {}: {}", 
        from_country, from_multi, 
        to_country, to_multi);

    // Get primary currencies
    let from_primary = GLOBAL_DATA.get().unwrap().get_primary_currency(&from_country).await;
    let to_primary = GLOBAL_DATA.get().unwrap().get_primary_currency(&to_country).await;

    debug!("Primary currencies - From: {:?}, To: {:?}", from_primary, to_primary);

    // Get available currencies if needed
    let available_currencies = if multiple_currencies_available {
        Some(get_available_currencies(
            from_currencies,
            to_currencies,
            &from_primary,
            &to_primary
        ))
    } else {
        None
    };

    // Select currencies
    let (from_currency_code, from_currency_info) = select_currency(
        from_currencies,
        request.preferred_currency.as_deref(),
        from_primary.as_deref(),
        &from_country,
    ).map_err(Error::from)?;

    let (to_currency_code, to_currency_info) = select_currency(
        to_currencies,
        request.preferred_currency.as_deref(),
        to_primary.as_deref(),
        &to_country,
    ).map_err(Error::from)?;

    debug!("Selected currencies - From: {}, To: {}", from_currency_code, to_currency_code);

    // Handle same currency case
    if from_currency_code == to_currency_code {
        debug!("Same currency conversion, returning original amount");
        return Ok(web::Json(DetailedConversionResponse {
            request_id,
            timestamp: Utc::now(),
            data: ConversionData {
                from: CurrencyDetails {
                    country: from_country_info.name.common,
                    currency_code: from_currency_code.to_string(),
                    currency_name: from_currency_info.name.clone(),
                    currency_symbol: from_currency_info.symbol.clone(),
                    amount: request.amount,
                    is_primary: from_primary.as_deref() == Some(from_currency_code),
                },
                to: CurrencyDetails {
                    country: to_country_info.name.common,
                    currency_code: to_currency_code.to_string(),
                    currency_name: to_currency_info.name.clone(),
                    currency_symbol: to_currency_info.symbol.clone(),
                    amount: request.amount,
                    is_primary: to_primary.as_deref() == Some(to_currency_code),
                },
                exchange_rate: 1.0,
                last_updated: Utc::now(),
                available_currencies,
            },
            meta: ResponseMetadata {
                source: "exchangerate-api.com".to_string(),
                response_time_ms: start_time.elapsed().as_millis() as u64,
                multiple_currencies_available,
                cache_hit: Some(true),
                rate_limit_remaining: None,
            },
        }));
    }

    // Get exchange rates and perform conversion
    let response = match http_client.get_exchange_rate(from_currency_code).await {
        Ok(r) => {
            debug!("Got exchange rate response: {:?}", r);
            r
        },
        Err(e) => {
            error!("Failed to get exchange rate: {}", e);
            return Ok(web::Json(DetailedConversionResponse {
                request_id,
                timestamp: Utc::now(),
                data: ConversionData {
                    from: CurrencyDetails {
                        country: from_country_info.name.common,
                        currency_code: from_currency_code.to_string(),
                        currency_name: from_currency_info.name.clone(),
                        currency_symbol: from_currency_info.symbol.clone(),
                        amount: request.amount,
                        is_primary: from_primary.as_deref() == Some(from_currency_code),
                    },
                    to: CurrencyDetails {
                        country: to_country_info.name.common,
                        currency_code: to_currency_code.to_string(),
                        currency_name: to_currency_info.name.clone(),
                        currency_symbol: to_currency_info.symbol.clone(),
                        amount: 0.0,
                        is_primary: to_primary.as_deref() == Some(to_currency_code),
                    },
                    exchange_rate: 0.0,
                    last_updated: Utc::now(),
                    available_currencies,
                },
                meta: ResponseMetadata {
                    source: "exchangerate-api.com".to_string(),
                    response_time_ms: start_time.elapsed().as_millis() as u64,
                    multiple_currencies_available,
                    cache_hit: Some(false),
                    rate_limit_remaining: None,
                },
            }));
        }
    };

    let rate = match response.conversion_rates.get(to_currency_code) {
        Some(r) => *r,
        None => {
            error!("Exchange rate not found for {}->{}", from_currency_code, to_currency_code);
            return Ok(web::Json(DetailedConversionResponse {
                request_id,
                timestamp: Utc::now(),
                data: ConversionData {
                    from: CurrencyDetails {
                        country: from_country_info.name.common,
                        currency_code: from_currency_code.to_string(),
                        currency_name: from_currency_info.name.clone(),
                        currency_symbol: from_currency_info.symbol.clone(),
                        amount: request.amount,
                        is_primary: from_primary.as_deref() == Some(from_currency_code),
                    },
                    to: CurrencyDetails {
                        country: to_country_info.name.common,
                        currency_code: to_currency_code.to_string(),
                        currency_name: to_currency_info.name.clone(),
                        currency_symbol: to_currency_info.symbol.clone(),
                        amount: 0.0,
                        is_primary: to_primary.as_deref() == Some(to_currency_code),
                    },
                    exchange_rate: 0.0,
                    last_updated: Utc::now(),
                    available_currencies,
                },
                meta: ResponseMetadata {
                    source: "exchangerate-api.com".to_string(),
                    response_time_ms: start_time.elapsed().as_millis() as u64,
                    multiple_currencies_available,
                    cache_hit: Some(false),
                    rate_limit_remaining: None,
                },
            }));
        }
    };

    let converted_amount = (request.amount * rate * 100.0).round() / 100.0;
    
    info!(
        "Conversion successful: {} {} -> {} {} (rate: {})",
        request.amount, from_currency_code, converted_amount, to_currency_code, rate
    );
    
    Ok(web::Json(DetailedConversionResponse {
        request_id,
        timestamp: Utc::now(),
        data: ConversionData {
            from: CurrencyDetails {
                country: from_country_info.name.common,
                currency_code: from_currency_code.to_string(),
                currency_name: from_currency_info.name.clone(),
                currency_symbol: from_currency_info.symbol.clone(),
                amount: request.amount,
                is_primary: from_primary.as_deref() == Some(from_currency_code),
            },
            to: CurrencyDetails {
                country: to_country_info.name.common,
                currency_code: to_currency_code.to_string(),
                currency_name: to_currency_info.name.clone(),
                currency_symbol: to_currency_info.symbol.clone(),
                amount: converted_amount,
                is_primary: to_primary.as_deref() == Some(to_currency_code),
            },
            exchange_rate: rate,
            last_updated: Utc::now(),
            available_currencies,
        },
        meta: ResponseMetadata {
            source: "exchangerate-api.com".to_string(),
            response_time_ms: start_time.elapsed().as_millis() as u64,
            multiple_currencies_available,
            cache_hit: Some(false),
            rate_limit_remaining: None,
        },
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
    currencies
        .iter()
        .next()
        .map(|(code, info)| {
            debug!("Falling back to first available currency {} for {}", code, country_name);
            (code.as_str(), info)
        })
        .ok_or_else(|| ServiceError::InvalidCurrency(
            format!("No valid currency found for {}", country_name)
        ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;
    use actix_web::{test, web::Data};
    use serde_json::json;
    use crate::models::DetailedErrorResponse;

    static INIT: Once = Once::new();

    fn setup() {
        INIT.call_once(|| {
            env::set_var("RUST_LOG", "debug");
            env_logger::init();
        });
    }

    #[actix_web::test]
    async fn test_multi_currency_conversion() {
        setup();
        
        let app = test::init_service(
            actix_web::App::new()
                .app_data(Data::new(Client::new()))
                .service(web::resource("/v1/currency")
                    .route(web::post().to(convert_currency)))
        ).await;

        env::set_var("EXCHANGE_RATE_API_KEY", "test_key");

        let req = test::TestRequest::post()
            .uri("/v1/currency")
            .insert_header(("content-type", "application/json"))
            .set_payload(json!({
                "from": "Panama",
                "to": "France",
                "amount": 100.0,
                "preferred_currency": "USD"
            }).to_string())
            .to_request();

        let resp = test::call_service(&app, req).await;
        let status = resp.status();
        assert!(status.is_success(), "Response status: {}, expected success", status);

        let body: DetailedConversionResponse = test::read_body_json(resp).await;
        assert_eq!(body.data.from.currency_code, "USD");
        assert!(body.meta.multiple_currencies_available);
        assert!(body.data.available_currencies.is_some());
    }

    #[actix_web::test]
    async fn test_invalid_preferred_currency() {
        setup();
        
        let app = test::init_service(
            actix_web::App::new()
                .app_data(Data::new(Client::new()))
                .service(web::resource("/v1/currency")
                    .route(web::post().to(convert_currency)))
        ).await;

        let req = test::TestRequest::post()
            .uri("/v1/currency")
            .insert_header(("content-type", "application/json"))
            .set_payload(json!({
                "from": "Panama",
                "to": "France",
                "amount": 100.0,
                "preferred_currency": "XYZ"
            }).to_string())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 400);

        let body: DetailedErrorResponse = test::read_body_json(resp).await;
        assert!(body.error.contains("Preferred currency"));
        assert!(body.error.contains("not available"));
    }

    #[actix_web::test]
    async fn test_same_currency() {
        setup();
        
        let app = test::init_service(
            actix_web::App::new()
                .app_data(Data::new(Client::new()))
                .service(web::resource("/v1/currency")
                    .route(web::post().to(convert_currency)))
        ).await;

        let req = test::TestRequest::post()
            .uri("/v1/currency")
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

        let body: DetailedConversionResponse = test::read_body_json(resp).await;
        assert_eq!(body.data.exchange_rate, 1.0);
        assert_eq!(body.data.to.amount, body.data.from.amount);
    }

    #[actix_web::test]
    async fn test_validation_errors() {
        setup();
        
        let app = test::init_service(
            actix_web::App::new()
                .app_data(Data::new(Client::new()))
                .service(web::resource("/v1/currency")
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
                .uri("/v1/currency")
                .insert_header(("content-type", "application/json"))
                .set_payload(payload.to_string())
                .to_request();

            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status().as_u16(), 400);

            let body: DetailedErrorResponse = test::read_body_json(resp).await;
            assert!(body.error.contains(expected_error));
        }
    }

    #[tokio::test]
    async fn test_select_currency() {
        setup();
        
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
}