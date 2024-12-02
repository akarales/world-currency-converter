use actix_web::test;
use currency_converter::{init_test_service, with_timeout};
use std::sync::Arc;

pub async fn test_health_endpoint() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    with_timeout(async {
        let app = init_test_service().await;
        
        let req = test::TestRequest::get()
            .uri("/health")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        
        let body = test::read_body(resp).await;
        assert_eq!(body, "OK");
        
        Ok(())
    }).await
}

pub async fn test_health_under_load() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    with_timeout(async {
        let app = Arc::new(init_test_service().await);
        
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let app = Arc::clone(&app);
                tokio::spawn(async move {
                    let req = test::TestRequest::get()
                        .uri("/health")
                        .to_request();
                    let resp = test::call_service(&app, req).await;
                    assert!(resp.status().is_success());
                    Ok(())
                })
            })
            .collect();

        for handle in handles {
            handle.await??;
        }

        Ok(())
    }).await
}