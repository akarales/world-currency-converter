use std::time::Duration;
use tokio::sync::OnceCell;
use std::sync::Once;
use log::debug;
use actix_web::{web, App, test, dev};
use async_trait::async_trait;
use actix_web::body::BoxBody;
use std::sync::Arc;

// Constants
pub const TEST_TIMEOUT: Duration = Duration::from_secs(5);
static INIT: Once = Once::new();
pub static TEST_INIT: OnceCell<()> = OnceCell::const_new();

pub mod mocks;
pub mod setup;

// Core test utilities
pub async fn with_timeout<F, T>(f: F) -> T 
where
    F: std::future::Future<Output = T>,
{
    match tokio::time::timeout(TEST_TIMEOUT, f).await {
        Ok(result) => result,
        Err(_) => panic!("Test timed out after {} seconds", TEST_TIMEOUT.as_secs()),
    }
}

pub async fn init_test_env() {
    TEST_INIT.get_or_init(|| async {
        INIT.call_once(|| {
            std::env::set_var("RUST_LOG", "debug");
            if std::env::var("EXCHANGE_RATE_API_KEY").is_err() {
                std::env::set_var("EXCHANGE_RATE_API_KEY", "test_key");
            }
            if std::env::var("CURRENCY_BACKUP_ENABLED").is_err() {
                std::env::set_var("CURRENCY_BACKUP_ENABLED", "true");
            }
            if std::env::var("CURRENCY_CONFIG_DIR").is_err() {
                std::env::set_var("CURRENCY_CONFIG_DIR", "config/test");
            }
            env_logger::builder()
                .is_test(true)
                .try_init()
                .ok();
        });

        ensure_test_dirs();
        setup::init_test_data().await;
    }).await;
}

pub fn ensure_test_dirs() {
    use std::fs;
    let test_dirs = ["config/test", "config/test/backups"];
    for dir in test_dirs {
        fs::create_dir_all(dir).unwrap_or_else(|e| {
            panic!("Failed to create test directory {}: {}", dir, e);
        });
    }
}

pub async fn init_test_service() -> impl dev::Service
    dev::ServiceRequest, 
    Response = dev::ServiceResponse<BoxBody>,
    Error = actix_web::Error
> + Clone {
    let client = Arc::new(
        reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client")
    );

    test::init_service(
        App::new()
            .app_data(web::Data::new(client))
            .service(web::resource("/health").route(web::get().to(crate::health_check)))
            .service(web::resource("/currency").route(web::post().to(crate::handlers::convert_currency)))
            .service(web::resource("/v1/currency").route(web::post().to(crate::handlers_v1::convert_currency)))
    ).await
}

// pub fn init_test_request(uri: &str) -> test::TestRequest {
//     test::TestRequest::with_uri(uri)
// }

#[derive(Clone)]
pub struct TestContext {
    pub app: Arc<Box<dyn dev::Service
        dev::ServiceRequest,
        Response = dev::ServiceResponse<BoxBody>,
        Error = actix_web::Error,
        Future = futures::future::BoxFuture<'static, Result<dev::ServiceResponse<BoxBody>, actix_web::Error>>
    > + Send + Sync + 'static>>
}

#[async_trait::async_trait]
pub trait TestService {
    async fn new() -> Self;
    async fn call_service(&self, request: test::TestRequest) -> dev::ServiceResponse<BoxBody>;
}

#[async_trait::async_trait]
impl TestService for TestContext {
    async fn new() -> Self {
        let app = Arc::new(Box::new(init_test_service().await));
        Self { app }
    }

    async fn call_service(&self, request: test::TestRequest) -> dev::ServiceResponse<BoxBody> {
        test::call_service(self.app.as_ref().as_ref(), request.to_request()).await
    }
}

pub async fn init_test_ctx() -> TestContext {
    init_test_env().await;
    TestContext::new().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http::StatusCode;
    use actix_web::test;

    #[actix_web::test]
    async fn test_init_test_service() {
        let app = init_test_service().await;
        let req = test::TestRequest::get().uri("/health").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn test_test_context() {
        let ctx = TestContext::new().await;
        let req = test::TestRequest::get().uri("/health");
        let resp = ctx.call_service(req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_timeout_wrapper() {
        let result = with_timeout(async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            42
        }).await;
        assert_eq!(result, 42);

        let result = std::panic::catch_unwind(|| async {
            with_timeout(async {
                tokio::time::sleep(Duration::from_secs(10)).await;
                42
            }).await
        });
        assert!(result.is_err());
    }
}