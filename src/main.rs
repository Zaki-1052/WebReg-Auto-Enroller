mod config;
mod stats;
mod notifier;
mod utils;
mod webreg;
mod monitor;
mod enroll;
mod state;

use std::time::Duration;
use tokio::time::sleep;
use tokio::signal::ctrl_c;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::error::Error as StdError;
use log::info;
use chrono::Local;

use config::{CourseDetails, to_section_groups};
use state::{AppState, refresh_cookie};
use monitor::monitor_section_with_retry;
use enroll::try_enroll_with_retry;
use utils::setup_logging;

async fn run_monitor(
    state: Arc<Mutex<AppState>>,
    shutdown: tokio::sync::broadcast::Receiver<()>,
) -> Result<(), Box<dyn StdError + Send + Sync>> {
    let mut shutdown_rx = shutdown;
    let mut cookie_refresh_timer = tokio::time::interval(
        Duration::from_secs(state.lock().await.config.monitoring.cookie_refresh_interval)
    );

    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                info!("Received shutdown signal, stopping monitoring...");
                break;
            }
            _ = cookie_refresh_timer.tick() => {
                let mut state_guard = state.lock().await;
                if let Err(e) = refresh_cookie(&mut state_guard).await {
                    log::error!("Failed to refresh cookie: {:?}", e);
                    continue;
                }
            }
            _ = async {
                let polling_interval = {
                    let mut state_guard = state.lock().await;

                    // Skip monitoring if not connected
                    if !state_guard.is_connected {
                        let interval = state_guard.config.webreg.polling_interval;
                        drop(state_guard); // Release lock before sleeping
                        sleep(Duration::from_secs(interval)).await;
                        return;
                    }

                    // Clone all the values we need
                    let term = state_guard.term.clone();
                    let wrapper = state_guard.clone_wrapper();
                    let notifier = state_guard.notifier.clone();
                    let chem_config = state_guard.config.courses.chem.clone();
                    let bild_config = state_guard.config.courses.bild.clone();
                    let polling_interval = state_guard.config.webreg.polling_interval;
                    let seat_threshold = state_guard.config.monitoring.seat_threshold;

                    // Monitor CHEM sections
                    let chem_sections = match &chem_config {
                        CourseDetails::New(details) => details.sections.clone(),
                        CourseDetails::Legacy(details) => to_section_groups(details),
                    };

                    for section_group in &chem_sections {
                    // Monitor lecture section
                    if let Ok(Some(section_id)) = monitor_section_with_retry(
                        &wrapper,
                        &term,
                        &section_group.lecture,
                        &chem_config.department(),
                        &chem_config.course_code(),
                        polling_interval,
                        seat_threshold,
                        &notifier,
                    ).await {
                        state_guard.stats.enrollment_attempts += 1;
                        if let Ok(true) = try_enroll_with_retry(
                            &wrapper,
                            &term,
                            &section_id,
                            &chem_config.department(),
                            &chem_config.course_code(),
                            &section_group.lecture,
                            &notifier,
                            &mut state_guard.stats,
                        ).await {
                            state_guard.stats.successful_enrollments += 1;
                        }
                    }

                    // Monitor discussion sections
                    for discussion in &section_group.discussions {
                        if let Ok(Some(section_id)) = monitor_section_with_retry(
                            &wrapper,
                            &term,
                            discussion,
                            &chem_config.department(),
                            &chem_config.course_code(),
                            polling_interval,
                            seat_threshold,
                            &notifier,
                        ).await {
                            state_guard.stats.enrollment_attempts += 1;
                            if let Ok(true) = try_enroll_with_retry(
                                &wrapper,
                                &term,
                                &section_id,
                                &chem_config.department(),
                                &chem_config.course_code(),
                                discussion,
                                &notifier,
                                &mut state_guard.stats,
                            ).await {
                                state_guard.stats.successful_enrollments += 1;
                            }
                        }
                    }
                }

                // Monitor BILD sections
                let bild_sections = to_section_groups(&bild_config);

                for section_group in &bild_sections {
                    // Monitor lecture section
                    if let Ok(Some(section_id)) = monitor_section_with_retry(
                        &wrapper,
                        &term,
                        &section_group.lecture,
                        &bild_config.department,
                        &bild_config.course_code,
                        polling_interval,
                        seat_threshold,
                        &notifier,
                    ).await {
                        state_guard.stats.enrollment_attempts += 1;
                        if let Ok(true) = try_enroll_with_retry(
                            &wrapper,
                            &term,
                            &section_id,
                            &bild_config.department,
                            &bild_config.course_code,
                            &section_group.lecture,
                            &notifier,
                            &mut state_guard.stats,
                        ).await {
                            state_guard.stats.successful_enrollments += 1;
                        }
                    }

                    // Monitor discussion sections
                    for discussion in &section_group.discussions {
                        if let Ok(Some(section_id)) = monitor_section_with_retry(
                            &wrapper,
                            &term,
                            discussion,
                            &bild_config.department,
                            &bild_config.course_code,
                            polling_interval,
                            seat_threshold,
                            &notifier,
                        ).await {
                            state_guard.stats.enrollment_attempts += 1;
                            if let Ok(true) = try_enroll_with_retry(
                                &wrapper,
                                &term,
                                &section_id,
                                &bild_config.department,
                                &bild_config.course_code,
                                discussion,
                                &notifier,
                                &mut state_guard.stats,
                            ).await {
                                state_guard.stats.successful_enrollments += 1;
                            }
                        }
                    }
                }

                    let health = state_guard.check_health().await;
                    info!("Health status: {:?}", health);
                    state_guard.last_check_time = Local::now().to_string();

                    polling_interval
                }; // Lock is released here

                // Sleep without holding the lock
                sleep(Duration::from_secs(polling_interval)).await;
            } => {}
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn StdError + Send + Sync>> {
    println!("Starting main...");

    // Setup logging
    println!("Setting up logging...");
    setup_logging()?;
    info!("Starting WebReg monitoring system...");

    // Initialize application state
    println!("Initializing application state...");
    let state = Arc::new(Mutex::new(AppState::new().await?));
    println!("Application state initialized successfully");

    // Setup shutdown channel
    println!("Setting up shutdown channel...");
    let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);

    // Print startup information
    {
        println!("Printing startup information...");
        let state_guard = state.lock().await;
        info!("Monitoring the following sections:");

        match &state_guard.config.courses.chem {
            CourseDetails::New(details) => {
                for section_group in &details.sections {
                    info!("CHEM {}: Lecture {} with discussions {:?}",
                        details.course_code,
                        section_group.lecture,
                        section_group.discussions);
                }
            },
            CourseDetails::Legacy(details) => {
                info!("CHEM {}: Lecture {} with discussions {:?}",
                    details.course_code,
                    details.lecture_section,
                    details.discussion_sections);
            }
        }

        info!("BILD {}: Lecture {} with discussions {:?}",
            state_guard.config.courses.bild.course_code,
            state_guard.config.courses.bild.lecture_section,
            state_guard.config.courses.bild.discussion_sections);

        info!("Checking every {} seconds", state_guard.config.webreg.polling_interval);
        info!("Press Ctrl+C to stop the program");
    }

    // Spawn monitoring task
    println!("Spawning monitoring task...");
    let monitor_handle = tokio::spawn(run_monitor(state.clone(), shutdown_rx));
    println!("Monitor task spawned, waiting for Ctrl+C...");

    // Wait for Ctrl+C
    tokio::select! {
        _ = ctrl_c() => {
            println!("Received Ctrl+C signal");
            info!("Received Ctrl+C, initiating graceful shutdown...");
            let _ = shutdown_tx.send(());
        }
    }

    // Wait for monitor to finish
    println!("Waiting for monitor to finish...");
    monitor_handle.await??;

    // Final stats update
    {
        println!("Updating final stats...");
        let mut state_guard = state.lock().await;
        state_guard.update_stats();
        let health = state_guard.check_health().await;
        info!("Final health status: {:?}", health);
    }

    println!("Shutdown complete!");
    info!("Shutdown complete!");
    Ok(())
}
