use actix_web::{test, web};
use currency_converter::main;

#[actix_web::test]
async fn test_health_check() {
    let app = test::init_service(
        actix_web::App::new()
            .service(web::resource("/health").route(web::get().to(main::health_check)))
    ).await;

    let req = test::TestRequest::get()
        .uri("/health")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body = test::read_body(resp).await;
    assert_eq!(body, "OK");
}