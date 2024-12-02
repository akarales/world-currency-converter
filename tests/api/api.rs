use actix_web::{test, web, App};
use currency_converter::{handlers, handlers_v1, models::*, data::{GLOBAL_DATA, GlobalData}};
use log::debug;
use serde_json::json;
use tokio::sync::OnceCell;
use std::env;
use std::time::Duration;

static TEST_ENV_INIT: OnceCell<()> = OnceCell::const_new();
static API_KEY_STORAGE: OnceCell<String> = OnceCell::const_new();

async fn save_api_key() {
    if let Ok(key) = env::var("EXCHANGE_RATE_API_KEY") {
        let _ = API_KEY_STORAGE.set(key);
    }
}

async fn setup_test_env() {
    let _ = TEST_ENV_INIT.get_or_init(|| async {
        std::env::set_var("RUST_LOG", "debug");
        env_logger::init();

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

            panic!("No exchange rate API key found. Please ensure one of the following:
                1. EXCHANGE_RATE_API_KEY is set in the environment
                2. .env file exists with EXCHANGE_RATE_API_KEY
                3. .env.test file exists with EXCHANGE_RATE_API_KEY
                4. .env.testing file exists with EXCHANGE_RATE_API_KEY");
        }
    }).await;
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
        .timeout(Duration::from_secs(30))
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
    setup_test_env().await;
    save_api_key().await;
    
    GLOBAL_DATA.get_or_init(|| async {
        GlobalData::new(API_KEY_STORAGE.get().unwrap().clone())
    }).await;

    let app = test::init_service(build_test_app()).await;

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
async fn test_simple_endpoint_invalid_country() {
    setup_test_env().await;
    save_api_key().await;
    
    GLOBAL_DATA.get_or_init(|| async {
        GlobalData::new(API_KEY_STORAGE.get().unwrap().clone())
    }).await;

    let app = test::init_service(build_test_app()).await;

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
async fn test_case_sensitivity() {
    setup_test_env().await;
    save_api_key().await;
    
    GLOBAL_DATA.get_or_init(|| async {
        GlobalData::new(API_KEY_STORAGE.get().unwrap().clone())
    }).await;

    let app = test::init_service(build_test_app()).await;

    let req = test::TestRequest::post()
        .uri("/currency")
        .insert_header(("content-type", "application/json"))
        .set_payload(json!({
            "from": "japan",
            "to": "australia",
            "amount": 1000.0,
            "preferred_currency": null
        }).to_string())
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: SimpleConversionResponse = test::read_body_json(resp).await;

    assert_eq!(body.from, "JPY");
    assert_eq!(body.to, "AUD");
    assert!(body.amount > 0.0);
}

#[actix_web::test]
async fn test_v1_endpoint_valid_conversion() {
    setup_test_env().await;
    save_api_key().await;
    
    GLOBAL_DATA.get_or_init(|| async {
        GlobalData::new(API_KEY_STORAGE.get().unwrap().clone())
    }).await;

    let app = test::init_service(build_test_app()).await;

    let req = test::TestRequest::post()
        .uri("/v1/currency")
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

    // Use bytes first to debug the response if needed
    let body_bytes = test::read_body(resp).await;
    let body_str = String::from_utf8(body_bytes.to_vec())
        .expect("Response was not valid UTF-8");
    
    let body: DetailedConversionResponse = serde_json::from_str(&body_str)
        .unwrap_or_else(|e| panic!("Failed to parse response: {}. Response was: {}", e, body_str));
    
    assert_eq!(body.data.from.currency_code, "USD");
    assert_eq!(body.data.to.currency_code, "EUR");
    assert!(body.data.to.amount > 0.0);
    assert!(body.data.exchange_rate > 0.0);
    assert!(!body.request_id.is_empty());
    assert_eq!(body.meta.source, "exchangerate-api.com");
}

#[actix_web::test]
async fn test_v1_endpoint_invalid_country() {
    setup_test_env().await;
    save_api_key().await;
    
    GLOBAL_DATA.get_or_init(|| async {
        GlobalData::new(API_KEY_STORAGE.get().unwrap().clone())
    }).await;

    let app = test::init_service(build_test_app()).await;

    let req = test::TestRequest::post()
        .uri("/v1/currency")
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
    assert!(!body.request_id.is_empty());
}

#[actix_web::test]
async fn test_service_errors() {
    setup_test_env().await;
    save_api_key().await;
    
    GLOBAL_DATA.get_or_init(|| async {
        GlobalData::new("invalid_key".to_string())
    }).await;

    let app = test::init_service(build_test_app()).await;

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
    assert_eq!(resp.status().as_u16(), 503);

    let body: DetailedErrorResponse = test::read_body_json(resp).await;
    assert!(body.error.contains("Service temporarily unavailable"));
}

#[actix_web::test]
async fn test_multi_currency_with_preferred() {
    setup_test_env().await;
    save_api_key().await;
    
    GLOBAL_DATA.get_or_init(|| async {
        GlobalData::new(API_KEY_STORAGE.get().unwrap().clone())
    }).await;

    let app = test::init_service(build_test_app()).await;

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
    assert!(resp.status().is_success());

    let body: DetailedConversionResponse = test::read_body_json(resp).await;
    
    assert_eq!(body.data.from.currency_code, "USD");
    assert_eq!(body.data.to.currency_code, "EUR");
    assert!(body.meta.multiple_currencies_available);
    assert!(body.data.available_currencies.is_some());
    
    if let Some(currencies) = body.data.available_currencies {
        assert!(currencies.iter().any(|c| c.code == "USD"));
        assert!(currencies.iter().any(|c| c.code == "PAB"));
    }
}

#[actix_web::test]
async fn test_invalid_preferred_currency() {
    setup_test_env().await;
    save_api_key().await;
    
    GLOBAL_DATA.get_or_init(|| async {
        GlobalData::new(API_KEY_STORAGE.get().unwrap().clone())
    }).await;

    let app = test::init_service(build_test_app()).await;

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
    assert!(body.error.contains("Currency not available"));
}

#[actix_web::test]
async fn test_validation_errors() {
    setup_test_env().await;
    save_api_key().await;
    
    GLOBAL_DATA.get_or_init(|| async {
        GlobalData::new(API_KEY_STORAGE.get().unwrap().clone())
    }).await;

    let app = test::init_service(build_test_app()).await;

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