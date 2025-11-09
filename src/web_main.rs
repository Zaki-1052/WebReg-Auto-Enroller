mod config;
mod stats;
mod notifier;
mod utils;
mod webreg;
mod monitor;
mod enroll;
mod state;
mod api;
mod web_server;
mod job_manager;

use std::sync::Arc;
use tokio::sync::Mutex;
use std::error::Error as StdError;
use log::info;

use state::AppState;
use utils::setup_logging;
use web_server::start_web_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn StdError + Send + Sync>> {
    println!("Starting WebReg Auto-Enroller Web Server...");

    // Setup logging
    setup_logging()?;
    info!("Starting WebReg Auto-Enroller Web Server...");

    // Initialize application state
    let state = Arc::new(Mutex::new(AppState::new().await?));

    // Start web server
    let port = 3000;
    start_web_server(state, port).await?;

    Ok(())
}
