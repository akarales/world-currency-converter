use actix_web::{web, App, HttpServer};
use currency_converter::{
    handlers, handlers_v1,
    cache::{Cache, ExchangeRateData},
    config::Config,
    registry::ServiceRegistry,
};
use dotenv::dotenv;
use log::{info, error, debug};
use std::{io, time::Duration};

async fn health_check() -> actix_web::Result<&'static str> {
    Ok("OK")
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    // Initialize environment and logging
    dotenv().ok();
    env_logger::init();
    
    // Load configuration
    let config = Config::new().map_err(|e| {
        error!("Failed to load configuration: {}", e);
        io::Error::new(io::ErrorKind::Other, e)
    })?;

    // Initialize service registry
    let registry = ServiceRegistry::new(&config).map_err(|e| {
        error!("Failed to initialize services: {}", e);
        io::Error::new(io::ErrorKind::Other, e)
    })?;
    
    let registry = web::Data::new(registry);
    
    // Initialize caches
    let exchange_rate_cache = web::Data::new(ExchangeRateData::new_cache());
    let country_cache = web::Data::new(Cache::<String>::new(
        24 * 60, // 24 hours TTL
        500      // Maximum number of country entries to cache
    ));

    // Start cache cleanup task
    let cleanup_exchange_rate_cache = exchange_rate_cache.clone();
    let cleanup_country_cache = country_cache.clone();
    tokio::spawn(async move {
        let cleanup_interval = Duration::from_secs(300); // 5 minutes
        start_cache_cleanup(
            cleanup_exchange_rate_cache,
            cleanup_country_cache,
            cleanup_interval
        ).await;
    });

    info!("Starting currency converter service at http://localhost:8080");
    
    // Start HTTP server
    HttpServer::new(move || {
        App::new()
            // Add registry
            .app_data(registry.clone())
            // Add shared services
            .app_data(exchange_rate_cache.clone())
            .app_data(country_cache.clone())
            
            // Health check endpoint
            .service(
                web::resource("/health")
                    .route(web::get().to(health_check))
            )
            
            // API v1 routes
            .service(
                web::scope("/v1")
                    .configure(configure_v1_routes)
            )
            
            // Legacy routes (without version prefix)
            .service(
                web::resource("/currency")
                    .route(web::post().to(handlers::convert_currency))
            )
    })
    .bind("127.0.0.1:8080")?
    .workers(4)
    .run()
    .await
}

fn configure_v1_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/currency")
            .route("", web::post().to(handlers_v1::convert_currency))
    );
}

async fn start_cache_cleanup(
    exchange_rate_cache: web::Data<Cache<ExchangeRateData>>,
    country_cache: web::Data<Cache<String>>,
    cleanup_interval: Duration
) {
    loop {
        tokio::time::sleep(cleanup_interval).await;
        debug!("Running periodic cache cleanup");
        
        exchange_rate_cache.clear_expired().await;
        country_cache.clear_expired().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[actix_web::test]
    async fn test_health_check() {
        let resp = health_check().await.unwrap();
        assert_eq!(resp, "OK");
    }
}