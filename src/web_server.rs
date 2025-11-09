use axum::Router;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::services::ServeDir;
use tower_http::cors::{CorsLayer, Any};

use crate::api::{create_router, ApiState};
use crate::state::AppState;
use crate::job_manager::JobManager;

pub async fn start_web_server(
    app_state: Arc<Mutex<AppState>>,
    port: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let job_manager = Arc::new(JobManager::new(app_state));

    let api_state = Arc::new(ApiState {
        job_manager,
    });

    // Create API router
    let api_router = create_router(api_state);

    // Create CORS layer
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Serve static files from the "static" directory
    let serve_dir = ServeDir::new("static");

    // Combine routes
    let app = Router::new()
        .merge(api_router)
        .nest_service("/", serve_dir)
        .layer(cors);

    // Start server
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    log::info!("Web server listening on http://{}", addr);
    println!("🌐 Web UI available at: http://localhost:{}", port);

    axum::serve(listener, app).await?;

    Ok(())
}
