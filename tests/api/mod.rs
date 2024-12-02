use actix_web::{test, web, App, dev::Service, dev::ServiceRequest, body::BoxBody, dev::ServiceResponse};

pub async fn setup_test_app() -> impl Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = actix_web::Error> {
    test::init_service(
        App::new()
            .service(web::resource("/currency")
                .route(web::post().to(currency_converter::handlers::convert_currency)))
            .service(web::resource("/health")
                .route(web::get().to(currency_converter::handlers::health_check)))
    ).await
}

// Helper function to create proper test requests
pub fn create_test_request(method: test::TestRequest) -> ServiceRequest {
    method.to_srv_request()
}

pub mod currency;
pub mod health;