use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;
use tokio::time::sleep;
use log::{info, error};
use chrono::Local;

use crate::state::AppState;
use crate::monitor::monitor_section_with_retry;
use crate::enroll::try_enroll_with_retry;
use crate::config::{CourseDetails, to_section_groups};

pub struct JobManager {
    pub state: Arc<Mutex<AppState>>,
    pub is_running: Arc<Mutex<bool>>,
    pub shutdown_tx: tokio::sync::broadcast::Sender<()>,
}

impl JobManager {
    pub fn new(state: Arc<Mutex<AppState>>) -> Self {
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);

        Self {
            state,
            is_running: Arc::new(Mutex::new(false)),
            shutdown_tx,
        }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut is_running = self.is_running.lock().await;

        if *is_running {
            return Err("Monitoring is already running".into());
        }

        *is_running = true;
        drop(is_running);

        let state = self.state.clone();
        let is_running_clone = self.is_running.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tokio::spawn(async move {
            let mut cookie_refresh_timer = {
                let state_guard = state.lock().await;
                tokio::time::interval(Duration::from_secs(
                    state_guard.config.monitoring.cookie_refresh_interval,
                ))
            };

            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        info!("Received shutdown signal, stopping monitoring...");
                        let mut running = is_running_clone.lock().await;
                        *running = false;
                        break;
                    }
                    _ = cookie_refresh_timer.tick() => {
                        let mut state_guard = state.lock().await;
                        if let Err(e) = crate::state::refresh_cookie(&mut state_guard).await {
                            log::error!("Failed to refresh cookie: {:?}", e);
                            continue;
                        }
                    }
                    _ = async {
                        let running = is_running_clone.lock().await;
                        if !*running {
                            return;
                        }
                        drop(running);

                        let mut state_guard = state.lock().await;

                        // Skip monitoring if not connected
                        if !state_guard.is_connected {
                            let polling_interval = state_guard.config.webreg.polling_interval;
                            drop(state_guard);
                            sleep(Duration::from_secs(polling_interval)).await;
                            return;
                        }

                        // Clone all the values we need
                        let term = state_guard.term.clone();
                        let polling_interval_val = state_guard.config.webreg.polling_interval;
                        let wrapper = match state_guard.clone_wrapper() {
                            Ok(w) => w,
                            Err(e) => {
                                error!("Failed to clone WebRegWrapper: {:?}", e);
                                drop(state_guard);
                                sleep(Duration::from_secs(polling_interval_val)).await;
                                return;
                            }
                        };
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
                            )
                            .await
                            {
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
                                )
                                .await
                                {
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
                                )
                                .await
                                {
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
                                    )
                                    .await
                                    {
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
                            )
                            .await
                            {
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
                                )
                                .await
                                {
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
                                )
                                .await
                                {
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
                                    )
                                    .await
                                    {
                                        state_guard.stats.successful_enrollments += 1;
                                    }
                                }
                            }
                        }

                        let health = state_guard.check_health().await;
                        info!("Health status: {:?}", health);
                        state_guard.last_check_time = Local::now().to_string();

                        // Release lock before sleeping to allow cookie refresh and API calls
                        drop(state_guard);

                        sleep(Duration::from_secs(polling_interval)).await;
                    } => {}
                }
            }
        });

        Ok(())
    }

    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let is_running = self.is_running.lock().await;

        if !*is_running {
            return Err("Monitoring is not running".into());
        }

        let _ = self.shutdown_tx.send(());

        Ok(())
    }

    pub async fn is_running(&self) -> bool {
        *self.is_running.lock().await
    }
}
