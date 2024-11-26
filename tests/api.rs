use actix_web::{test, web, App};
use currency_converter::{handlers, handlers_v1, models::*};
use log::debug;
use serde_json::json;
use std::sync::{Arc, Mutex, Once};
use std::env;

static INIT: Once = Once::new();
static API_KEY_INIT: Once = Once::new();
static mut ORIGINAL_API_KEY: Option<Arc<Mutex<Option<String>>>> = None;

fn save_api_key() -> Arc<Mutex<Option<String>>> {
    let key_storage = Arc::new(Mutex::new(None));
    unsafe {
        API_KEY_INIT.call_once(|| {
            ORIGINAL_API_KEY = Some(key_storage.clone());
        });
        if let Some(original) = env::var("EXCHANGE_RATE_API_KEY").ok() {
            *key_storage.lock().unwrap() = Some(original);
        }
    }
    key_storage
}

fn restore_api_key(key_storage: &Arc<Mutex<Option<String>>>) {
    if let Some(key) = key_storage.lock().unwrap().as_ref() {
        env::set_var("EXCHANGE_RATE_API_KEY", key);
    }
}

/// Ensures test environment is properly configured
fn setup_test_env() {
    INIT.call_once(|| {
        std::env::set_var("RUST_LOG", "debug");
        env_logger::init();
    });

    // Try to load API key from different sources
    if env::var("EXCHANGE_RATE_API_KEY").is_err() {
        // Try loading from .env files
        for env_file in &[".env", ".env.test", ".env.testing"] {
            if dotenv::from_filename(env_file).is_ok() {
                if env::var("EXCHANGE_RATE_API_KEY").is_ok() {
                    debug!("Loaded API key from {}", env_file);
                    return;
                }
            }
        }

        // If no API key is found, panic with helpful message
        panic!("No exchange rate API key found. Please ensure one of the following:
            1. EXCHANGE_RATE_API_KEY is set in the environment
            2. .env file exists with EXCHANGE_RATE_API_KEY
            3. .env.test file exists with EXCHANGE_RATE_API_KEY
            4. .env.testing file exists with EXCHANGE_RATE_API_KEY");
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
    let key_storage = save_api_key();
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
    let status = resp.status();
    
    assert!(
        status.is_success(),
        "Response status: {}, expected success",
        status
    );

    let body: SimpleConversionResponse = test::read_body_json(resp).await;
    restore_api_key(&key_storage);
    
    assert_eq!(body.from, "USD");
    assert_eq!(body.to, "EUR");
    assert!(body.amount > 0.0);
}

#[actix_web::test]
async fn test_simple_endpoint_invalid_country() {
    setup_test_env();
    let key_storage = save_api_key();
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
    restore_api_key(&key_storage);
    
    assert_eq!(body.from, "INVALID");
    assert_eq!(body.to, "INVALID");
    assert_eq!(body.amount, 0.0);
}

#[actix_web::test]
async fn test_case_sensitivity() {
    setup_test_env();
    let key_storage = save_api_key();
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
    assert!(resp.status().is_success());

    let body: SimpleConversionResponse = test::read_body_json(resp).await;
    restore_api_key(&key_storage);

    assert_eq!(body.from, "JPY");
    assert_eq!(body.to, "AUD");
    assert!(body.amount > 0.0);
}

#[actix_web::test]
async fn test_v1_endpoint_valid_conversion() {
    setup_test_env();
    let key_storage = save_api_key();
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
    assert!(resp.status().is_success());

    let body: DetailedConversionResponse = test::read_body_json(resp).await;
    restore_api_key(&key_storage);
    
    assert_eq!(body.data.from.currency_code, "USD");
    assert_eq!(body.data.to.currency_code, "EUR");
    assert!(body.data.to.amount > 0.0);
}

#[actix_web::test]
async fn test_v1_endpoint_invalid_country() {
    setup_test_env();
    let key_storage = save_api_key();
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
    restore_api_key(&key_storage);
    
    assert!(body.error.contains("Country not found: Narnia"));
}

#[actix_web::test]
async fn test_service_errors() {
    setup_test_env();
    let key_storage = save_api_key();
    let app = test::init_service(build_test_app()).await;

    // Remove API key to force error
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
    assert_eq!(resp.status().as_u16(), 503, "Expected 503 Service Unavailable when API key is missing");

    let body: SimpleConversionResponse = test::read_body_json(resp).await;
    restore_api_key(&key_storage);
    
    assert_eq!(body.from, "ERROR");
    assert_eq!(body.to, "ERROR");
    assert_eq!(body.amount, 0.0);
}