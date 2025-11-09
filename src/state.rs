use std::time::SystemTime;
use std::error::Error as StdError;
use std::path::Path;
use std::fs;
use std::collections::HashMap;
use webweg::wrapper::WebRegWrapper;
use chrono::Local;
use log::{info, error};
use crate::config::{AppConfig, CONFIG_PATH};
use crate::stats::{EnrollmentStats, HealthStatus};
use crate::notifier::Notifier;
use crate::webreg::{initialize_webreg, is_connection_valid};
use crate::monitor::monitor_section_with_retry;
use crate::utils::format_duration;

pub struct AppState {
    pub stats: EnrollmentStats,
    pub config: AppConfig,
    pub notifier: Notifier,
    pub wrapper: WebRegWrapper,
    pub start_time: SystemTime,
    pub last_check_time: String,
    pub is_connected: bool,
    pub term: String,
}

impl AppState {
    pub async fn new() -> Result<Self, Box<dyn StdError + Send + Sync>> {
        println!("Starting AppState::new()");

        if !Path::new(CONFIG_PATH).exists() {
            println!("Config file not found!");
            return Err("config.toml not found in current directory".into());
        }

        println!("Reading config file...");
        let config_content = fs::read_to_string(CONFIG_PATH)
            .map_err(|e| {
                println!("Error reading config: {:?}", e);
                format!("Failed to read config.toml: {}", e)
            })?;

        println!("Parsing config content...");
        println!("Config content: {}", config_content);

        let config: AppConfig = toml::from_str(&config_content)
            .map_err(|e| {
                println!("Error parsing TOML: {:?}", e);
                format!("Failed to parse config.toml: {}", e)
            })?;

        println!("Successfully parsed config");

        // Initialize stats with default values
        println!("Initializing stats...");
        let stats = EnrollmentStats {
            start_time: Local::now().to_string(),
            last_updated: Local::now().to_string(),
            total_checks: 0,
            openings_found: 0,
            enrollment_attempts: 0,
            successful_enrollments: 0,
            errors: 0,
            section_failures: HashMap::new(),
        };

        println!("Creating WebReg wrapper and notifier...");
        let term = config.webreg.term.clone();
        let notifier = Notifier::new(&config.notifications)?;
        let wrapper = initialize_webreg(&config.webreg).await?;

        println!("AppState::new() completed successfully");
        Ok(Self {
            stats,
            config,
            notifier,
            wrapper,
            start_time: SystemTime::now(),
            last_check_time: Local::now().to_string(),
            is_connected: true,
            term,
        })
    }

    pub fn clone_wrapper(&self) -> WebRegWrapper {
        WebRegWrapper::builder()
            .with_cookies(&self.config.webreg.cookie)
            .try_build_wrapper()
            .expect("Failed to clone WebRegWrapper")
    }

    pub fn update_stats(&mut self) {
        self.stats.last_updated = Local::now().to_string();
        let stats_json = serde_json::to_string_pretty(&self.stats).unwrap();
        if let Err(e) = fs::write(&self.config.monitoring.stats_file, stats_json) {
            error!("Failed to write stats file: {:?}", e);
        }
    }

    pub async fn check_health(&self) -> HealthStatus {
        let uptime = SystemTime::now()
            .duration_since(self.start_time)
            .unwrap_or_default();

        let success_rate = if self.stats.enrollment_attempts > 0 {
            (self.stats.successful_enrollments as f64 / self.stats.enrollment_attempts as f64) * 100.0
        } else {
            0.0
        };

        HealthStatus {
            uptime: format_duration(uptime),
            last_successful_check: self.last_check_time.clone(),
            connection_status: self.is_connected,
            error_count: self.stats.errors,
            success_rate,
            total_checks: self.stats.total_checks,
        }
    }

    pub async fn monitor_section_health(
        &mut self,
        section: &str,
        department: &str,
        course_code: &str,
    ) -> Result<Option<String>, Box<dyn StdError + Send + Sync>> {
        self.stats.total_checks += 1;
        let result = monitor_section_with_retry(
            &self.wrapper,
            &self.term,
            section,
            department,
            course_code,
            self.config.webreg.polling_interval,
            self.config.monitoring.seat_threshold,
            &self.notifier,
        ).await;

        match &result {
            Ok(Some(_)) => self.stats.openings_found += 1,
            Ok(None) => {},
            Err(_) => self.stats.errors += 1,
        }

        self.update_stats();
        result
    }
}

pub async fn refresh_cookie(state: &mut AppState) -> Result<(), Box<dyn StdError + Send + Sync>> {
    info!("Checking WebReg session status...");

    let is_valid = is_connection_valid(&state.wrapper, &state.term).await;

    if !is_valid && state.is_connected {
        // Cookie just expired (transition from connected to disconnected)
        state.is_connected = false;

        let msg = format!(
            "⚠️  WebReg Cookie has expired!\n\
            Time: {}\n\
            Please update the cookie in config.toml to resume monitoring.",
            Local::now().format("%Y-%m-%d %H:%M:%S")
        );

        state.notifier.send_notification(&msg).await;
        error!("WebReg cookie has expired!");
        return Err("Cookie expired".into());
    }

    if is_valid {
        state.is_connected = true;
        info!("WebReg session is valid");
    }

    Ok(())
}
