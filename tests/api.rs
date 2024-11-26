use actix_web::{test, web, App};
use currency_converter::{handlers, handlers_v1, models::*};
use log::debug;
use serde_json::json;
use once_cell::sync::Lazy;
use std::sync::Mutex;
use std::sync::Once;
use std::env;

static INIT: Once = Once::new();
static API_KEY: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

/// Ensures test environment is properly configured
fn setup_test_env() {
    INIT.call_once(|| {
        std::env::set_var("RUST_LOG", "debug");
        env_logger::init();
        
        // Save existing API key if present
        if let Ok(key) = env::var("EXCHANGE_RATE_API_KEY") {
            *API_KEY.lock().unwrap() = Some(key);
        } else {
            // Try loading from .env files
            for env_file in &[".env", ".env.test", ".env.testing"] {
                if dotenv::from_filename(env_file).is_ok() {
                    if let Ok(key) = env::var("EXCHANGE_RATE_API_KEY") {
                        *API_KEY.lock().unwrap() = Some(key);
                        debug!("Loaded API key from {}", env_file);
                        break;
                    }
                }
            }
        }
        
        // If still no API key found, panic with helpful message
        if API_KEY.lock().unwrap().is_none() {
            panic!("No exchange rate API key found. Please ensure one of the following:
                1. EXCHANGE_RATE_API_KEY is set in the environment
                2. .env file exists with EXCHANGE_RATE_API_KEY
                3. .env.test file exists with EXCHANGE_RATE_API_KEY
                4. .env.testing file exists with EXCHANGE_RATE_API_KEY");
        }
    });

    // Always ensure the API key is set before each test
    if let Some(key) = &*API_KEY.lock().unwrap() {
        env::set_var("EXCHANGE_RATE_API_KEY", key);
    }
}

fn build_test_app() -> actix_web::App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Response = actix_web::dev::ServiceResponse<actix_web::body::BoxBody>,
        Config = (),
        Error = actix_web::Error,
        InitError = ()
    >
> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client");
    
    let client_data = web::Data::new(client);

    App::new()
        .app_data(client_data.clone())
        .service(
            web::resource("/currency")
                .route(web::post().to(handlers::convert_currency))
        )
        .service(
            web::resource("/v1/currency")
                .route(web::post().to(handlers_v1::convert_currency))
        )
}

#[actix_web::test]
async fn test_simple_endpoint_valid_conversion() {
    setup_test_env();
    let app = test::init_service(build_test_app()).await;

    let req = test::TestRequest::post()
        .uri("/currency")
        .insert_header(("content-type", "application/json"))
        .set_payload(json!({
            "from": "United States",
            "to": "France",
            "amount": 100.0
        }).to_string())
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "Response status: {}, expected success",
        resp.status()
    );

    let body: SimpleConversionResponse = test::read_body_json(resp).await;
    
    assert_eq!(body.from, "USD");
    assert_eq!(body.to, "EUR");
    assert!(body.amount > 0.0);
}

#[actix_web::test]
async fn test_simple_endpoint_invalid_country() {
    setup_test_env();
    let app = test::init_service(build_test_app()).await;

    let req = test::TestRequest::post()
        .uri("/currency")
        .insert_header(("content-type", "application/json"))
        .set_payload(json!({
            "from": "Narnia",
            "to": "France",
            "amount": 100.0
        }).to_string())
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: SimpleConversionResponse = test::read_body_json(resp).await;
    
    assert_eq!(body.from, "INVALID");
    assert_eq!(body.to, "INVALID");
    assert_eq!(body.amount, 0.0);
}

#[actix_web::test]
async fn test_case_sensitivity() {
    setup_test_env();
    let app = test::init_service(build_test_app()).await;

    let req = test::TestRequest::post()
        .uri("/currency")
        .insert_header(("content-type", "application/json"))
        .set_payload(json!({
            "from": "japan",
            "to": "australia",
            "amount": 1000.0
        }).to_string())
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "Response status: {}, expected success",
        resp.status()
    );

    let body: SimpleConversionResponse = test::read_body_json(resp).await;

    assert_eq!(body.from, "JPY");
    assert_eq!(body.to, "AUD");
    assert!(body.amount > 0.0);
}

#[actix_web::test]
async fn test_v1_endpoint_valid_conversion() {
    setup_test_env();
    let app = test::init_service(build_test_app()).await;

    let req = test::TestRequest::post()
        .uri("/v1/currency")
        .insert_header(("content-type", "application/json"))
        .set_payload(json!({
            "from": "United States",
            "to": "France",
            "amount": 100.0
        }).to_string())
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "Response status: {}, expected success",
        resp.status()
    );

    let body: DetailedConversionResponse = test::read_body_json(resp).await;
    
    assert_eq!(body.data.from.currency_code, "USD");
    assert_eq!(body.data.to.currency_code, "EUR");
    assert!(body.data.to.amount > 0.0);
    assert!(body.data.exchange_rate > 0.0);
    assert!(!body.request_id.is_empty());
    assert_eq!(body.meta.source, "exchangerate-api.com");
}

#[actix_web::test]
async fn test_v1_endpoint_invalid_country() {
    setup_test_env();
    let app = test::init_service(build_test_app()).await;

    let req = test::TestRequest::post()
        .uri("/v1/currency")
        .insert_header(("content-type", "application/json"))
        .set_payload(json!({
            "from": "Narnia",
            "to": "France",
            "amount": 100.0
        }).to_string())
        .to_request();

    let resp = test::call_service(&app, req).await;
    
    assert_eq!(resp.status().as_u16(), 400);
    let body: DetailedErrorResponse = test::read_body_json(resp).await;
    
    assert!(body.error.contains("Country not found: Narnia"));
    assert!(!body.request_id.is_empty());
}

#[actix_web::test]
async fn test_service_errors() {
    setup_test_env();
    let app = test::init_service(build_test_app()).await;

    // Temporarily remove API key to force error
    let original_key = env::var("EXCHANGE_RATE_API_KEY").ok();
    env::remove_var("EXCHANGE_RATE_API_KEY");

    let req = test::TestRequest::post()
        .uri("/currency")
        .insert_header(("content-type", "application/json"))
        .set_payload(json!({
            "from": "United States",
            "to": "France",
            "amount": 100.0
        }).to_string())
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status().as_u16(),
        503,
        "Expected 503 Service Unavailable when API key is missing"
    );

    let body: SimpleConversionResponse = test::read_body_json(resp).await;
    
    assert_eq!(body.from, "ERROR");
    assert_eq!(body.to, "ERROR");
    assert_eq!(body.amount, 0.0);

    // Restore the API key
    if let Some(key) = original_key {
        env::set_var("EXCHANGE_RATE_API_KEY", key);
    }
}