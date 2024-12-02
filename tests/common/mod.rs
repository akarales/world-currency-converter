use std::time::Duration;
use tokio::time::timeout;
use currency_converter::models::{CountryInfo, CurrencyInfo};
use std::collections::HashMap;
use actix_web::{test, web, App, dev::ServiceResponse, body::BoxBody};
use currency_converter::{handlers, handlers_v1, health_check};

pub const TEST_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn with_timeout<F, T>(f: F) -> T 
where
    F: std::future::Future<Output = T>,
{
    timeout(TEST_TIMEOUT, f)
        .await
        .unwrap_or_else(|_| panic!("Test timed out after {} seconds", TEST_TIMEOUT.as_secs()))
}

pub fn create_test_currencies() -> HashMap<String, CurrencyInfo> {
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
    currencies
}

pub fn create_test_country(name: &str, currencies: Option<HashMap<String, CurrencyInfo>>) -> CountryInfo {
    CountryInfo {
        name: currency_converter::models::CountryName {
            common: name.to_string(),
            official: format!("Official {}", name),
            native_name: None,
        },
        currencies,
    }
}

pub async fn build_test_app() -> impl actix_web::dev::Service<
    actix_web::dev::ServiceRequest,
    Response = ServiceResponse<BoxBody>,
    Error = actix_web::Error,
> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client");
    
    let client_data = web::Data::new(client);

    test::init_service(
        App::new()
            .app_data(client_data.clone())
            .service(web::resource("/health").route(web::get().to(health_check)))
            .service(web::resource("/currency").route(web::post().to(handlers::convert_currency)))
            .service(web::resource("/v1/currency").route(web::post().to(handlers_v1::convert_currency)))
    ).await
}

pub async fn make_test_request(req: test::TestRequest) -> test::TestRequest {
    req
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http::StatusCode;

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

    #[test]
    fn test_currency_creation() {
        let currencies = create_test_currencies();
        assert!(currencies.contains_key("USD"));
        assert!(currencies.contains_key("EUR"));
        assert_eq!(currencies.get("USD").unwrap().symbol, "$");
        assert_eq!(currencies.get("EUR").unwrap().symbol, "€");
    }

    #[test]
    fn test_country_creation() {
        let currencies = create_test_currencies();
        let country = create_test_country("Test Country", Some(currencies.clone()));
        
        assert_eq!(country.name.common, "Test Country");
        assert!(country.currencies.is_some());
        assert_eq!(
            country.currencies.unwrap().get("USD").unwrap().symbol,
            currencies.get("USD").unwrap().symbol
        );
    }

    #[actix_web::test]
    async fn test_health_endpoint() {
        let app = build_test_app().await;
        let req = test::TestRequest::get().uri("/health").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }
}