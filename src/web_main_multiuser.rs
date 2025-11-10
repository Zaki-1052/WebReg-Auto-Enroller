// Multi-user web server entry point
mod config;
mod stats;
mod notifier;
mod utils;
mod webreg;
mod monitor;
mod enroll;
mod models;
mod db;
mod encryption;
mod auth;
mod multi_user_state;
mod multi_user_api;

use std::sync::Arc;
use std::error::Error as StdError;
use log::info;
use dotenv::dotenv;

use multi_user_state::MultiUserState;
use multi_user_api::{create_router, MultiUserApiState};
use encryption::EncryptionKey;
use utils::setup_logging;

#[tokio::main]
async fn main() -> Result<(), Box<dyn StdError + Send + Sync>> {
    println!("Starting WebReg Auto-Enroller Multi-User Web Server...");

    // Load environment variables
    dotenv().ok();

    // Setup logging
    setup_logging()?;
    info!("Starting WebReg Auto-Enroller Multi-User Web Server...");

    // Get database URL from environment
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in .env file");

    // Initialize database connection pool
    info!("Connecting to database...");
    let pool = db::init_pool(&database_url).await?;
    info!("Database connected successfully");

    // Initialize encryption key
    info!("Initializing encryption...");
    let encryption_key = EncryptionKey::from_env()?;
    info!("Encryption initialized");

    // Create multi-user state
    let state = Arc::new(MultiUserState::new(pool, encryption_key));

    // Create API state
    let api_state = Arc::new(MultiUserApiState { state });

    // Create router
    let app = create_router(api_state);

    // Add CORS middleware
    let app = app.layer(
        tower_http::cors::CorsLayer::new()
            .allow_origin(tower_http::cors::Any)
            .allow_methods(tower_http::cors::Any)
            .allow_headers(tower_http::cors::Any)
    );

    // Serve static files
    let app = app.nest_service(
        "/",
        tower_http::services::ServeDir::new("static")
    );

    // Get port from environment or use default
    let host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("SERVER_PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .expect("Invalid port number");

    let addr = format!("{}:{}", host, port);
    info!("Starting server on {}", addr);

    // Start server
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Server listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
