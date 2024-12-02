use actix_web::test;
use currency_converter::{
    init_test_service,
    models::SimpleConversionResponse,
    with_timeout,
};
use serde_json::json;

pub async fn test_currency_conversion_endpoint() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    with_timeout(async {
        let app = init_test_service().await;
        
        let req = test::TestRequest::post()
            .uri("/currency")
            .insert_header(("content-type", "application/json"))
            .set_json(json!({
                "from": "United States",
                "to": "France",
                "amount": 100.0,
                "preferred_currency": null
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body: SimpleConversionResponse = test::read_body_json(resp).await;
        assert_eq!(body.from, "USD");
        assert_eq!(body.to, "EUR");
        assert!(body.amount > 0.0);
        
        Ok(())
    }).await
}

pub async fn test_multi_currency_scenarios() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    with_timeout(async {
        let app = init_test_service().await;
        
        let req = test::TestRequest::post()
            .uri("/currency")
            .insert_header(("content-type", "application/json"))
            .set_json(json!({
                "from": "Panama",
                "to": "France",
                "amount": 100.0,
                "preferred_currency": "USD"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body: SimpleConversionResponse = test::read_body_json(resp).await;
        assert_eq!(body.from, "USD");
        assert_eq!(body.to, "EUR");
        
        Ok(())
    }).await
}