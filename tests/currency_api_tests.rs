use actix_web::test;
use currency_converter::{
    init_test_service,
    init_test_env,
    models::DetailedErrorResponse,
    with_timeout,
};
use serde_json::json;

mod api;

#[actix_web::test]
async fn run_api_tests() {
    init_test_env().await;

    let results = vec![
        ("Currency conversion", api::currency::test_currency_conversion_endpoint().await),
        ("Multi-currency test", api::currency::test_multi_currency_scenarios().await),
        ("Health check", api::health::test_health_endpoint().await),
        ("Load test", api::health::test_health_under_load().await)
    ];

    for (name, result) in results {
        match result {
            Ok(_) => println!("✓ {} passed", name),
            Err(e) => panic!("Test '{}' failed: {}", name, e),
        }
    }
}

#[actix_web::test]
async fn test_currency_api_error_handling() {
    with_timeout(async {
        init_test_env().await;
        let app = init_test_service().await;

        let test_cases = vec![
            (
                json!({
                    "from": "", 
                    "to": "France", 
                    "amount": 100.0,
                    "preferred_currency": null
                }),
                400,
                "Source country cannot be empty"
            ),
            (
                json!({
                    "from": "USA",
                    "to": "",
                    "amount": -1.0,
                    "preferred_currency": null
                }),
                400,
                "Amount must be greater than 0"
            ),
            (
                json!({
                    "from": "Narnia",
                    "to": "France",
                    "amount": 100.0,
                    "preferred_currency": null
                }),
                404,
                "Country not found"
            ),
        ];

        for (payload, expected_status, expected_error) in test_cases {
            let req = test::TestRequest::post()
                .uri("/currency")
                .insert_header(("content-type", "application/json"))
                .set_payload(payload.to_string())
                .to_request();

            let resp = test::call_service(&app, req).await;
            assert_eq!(
                resp.status().as_u16(), 
                expected_status,
                "Expected status {} for payload {:?}",
                expected_status,
                payload
            );

            let body: DetailedErrorResponse = test::read_body_json(resp).await;
            assert!(
                body.error.contains(expected_error),
                "Expected error '{}', got '{}'",
                expected_error,
                body.error
            );
        }
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }).await.unwrap();
}