use currency_converter::{
    handlers, handlers_v1,
    config::Config,
    registry::ServiceRegistry,
    data::{GLOBAL_DATA, GlobalData},
    currency_manager::CurrencyManager,
};
use actix_web::{web, App, HttpServer};
use dotenv::dotenv;
use log::{info, error, debug};
use std::{io, time::Duration};
use reqwest::Client;
use tokio::signal;

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

    // Initialize HTTP client with configured timeouts
    let client = Client::builder()
        .timeout(Duration::from_secs(config.api_settings.request_timeout_seconds))
        .connect_timeout(Duration::from_secs(config.api_settings.connect_timeout_seconds))
        .user_agent(&config.api_settings.user_agent)
        .build()
        .map_err(|e| {
            error!("Failed to create HTTP client: {}", e);
            io::Error::new(io::ErrorKind::Other, e)
        })?;

    // Initialize currency manager
    debug!("Initializing currency manager...");
    let currency_manager = CurrencyManager::new(
        config.exchange_rate_api_key.clone(),
        false // Not test mode
    );

    // Ensure currency data is ready
    if let Err(e) = currency_manager.ensure_currency_data().await {
        error!("Failed to initialize currency data: {}", e);
        return Err(io::Error::new(io::ErrorKind::Other, e));
    }
    info!("Currency data initialized successfully");

    // Initialize global data with API key
    GLOBAL_DATA.set(GlobalData::new(config.exchange_rate_api_key.clone()))
        .map_err(|_| {
            error!("Failed to initialize global data");
            io::Error::new(io::ErrorKind::Other, "Failed to initialize global data")
        })?;

    // Initialize global data
    info!("Initializing global country and currency data...");
    GLOBAL_DATA.get()
        .expect("Global data not initialized")
        .ensure_initialized()
        .await;
    info!("Global data initialization complete");

    let client_data = web::Data::new(client);
    
    // Initialize service registry
    let registry = ServiceRegistry::new(&config).map_err(|e| {
        error!("Failed to initialize services: {}", e);
        io::Error::new(io::ErrorKind::Other, e)
    })?;

    // Start background tasks
    registry.start_background_tasks(&config).await;

    let registry = web::Data::new(registry);
    
    info!("Starting currency converter service at http://localhost:8080");
    debug!("Using configuration: {:?}", config);
    
    // Create server
    let server = HttpServer::new(move || {
        App::new()
            // Add shared services
            .app_data(client_data.clone())
            .app_data(registry.clone())
            
            // Health check endpoint
            .service(
                web::resource("/health")
                    .route(web::get().to(currency_converter::health_check))
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
    .shutdown_timeout(30) // 30 seconds shutdown timeout
    .run();

    // Start the server
    let server_handle = server.handle();
    
    // Handle shutdown signals
    tokio::spawn(async move {
        handle_shutdown_signals(server_handle).await;
    });

    info!("Server started successfully");
    server.await
}

fn configure_v1_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/currency")
            .route("", web::post().to(handlers_v1::convert_currency))
    );
}

async fn handle_shutdown_signals(server: actix_web::dev::ServerHandle) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C signal");
        },
        _ = terminate => {
            info!("Received terminate signal");
        },
    }

    info!("Starting graceful shutdown (30s timeout)...");
    
    // Stop accepting new connections and perform cleanup
    server.stop(true).await;

    // Give the update service time to finish any pending updates
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    info!("Server shutdown completed");
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test;

    #[actix_web::test]
    async fn test_health_check() {
        let resp = currency_converter::health_check().await.unwrap();
        assert_eq!(resp, "OK");
    }

    #[actix_web::test]
    async fn test_server_configuration() {
        let config = Config::with_test_settings();
        let client = Client::new();
        let client_data = web::Data::new(client);
        let registry = ServiceRegistry::new(&config).unwrap();
        let registry_data = web::Data::new(registry);

        let app = test::init_service(
            App::new()
                .app_data(client_data)
                .app_data(registry_data)
                .service(web::resource("/health").route(web::get().to(currency_converter::health_check)))
        ).await;

        let req = test::TestRequest::get()
            .uri("/health")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }
}