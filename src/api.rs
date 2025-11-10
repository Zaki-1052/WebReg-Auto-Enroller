use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::config::SectionGroup;

// API Types
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CourseRequest {
    pub id: Option<String>,
    pub department: String,
    pub course_code: String,
    pub sections: Vec<SectionGroup>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JobConfig {
    pub term: String,
    pub polling_interval: u64,
    pub cookie: String,
    pub courses: Vec<CourseRequest>,
    pub seat_threshold: i64,
    pub monitoring_mode: MonitoringMode,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum MonitoringMode {
    Include,  // Only enroll when seats are available (seat_threshold = 0)
    Exclude,  // Only enroll when seats are limited (seat_threshold > 0)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JobResponse {
    pub job_id: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResponse {
    pub is_running: bool,
    pub is_connected: bool,
    pub last_check_time: String,
    pub stats: StatsResponse,
    pub health: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatsResponse {
    pub enrollment_attempts: u64,
    pub successful_enrollments: u64,
    pub errors: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotificationConfig {
    pub gmail_address: String,
    pub gmail_app_password: String,
    pub email_recipients: Vec<String>,
    pub discord_webhook_url: String,
}

use crate::job_manager::JobManager;

// Shared state for API
pub struct ApiState {
    pub job_manager: Arc<JobManager>,
}

// API Handlers
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "WebReg Auto-Enroller"
    }))
}

async fn get_status(State(state): State<Arc<ApiState>>) -> Result<Json<StatusResponse>, StatusCode> {
    let app_state = state.job_manager.state.lock().await;
    let is_running = state.job_manager.is_running().await;
    let health = app_state.check_health().await;

    Ok(Json(StatusResponse {
        is_running,
        is_connected: app_state.is_connected,
        last_check_time: app_state.last_check_time.clone(),
        stats: StatsResponse {
            enrollment_attempts: app_state.stats.enrollment_attempts,
            successful_enrollments: app_state.stats.successful_enrollments,
            errors: app_state.stats.errors,
        },
        health: format!("{:?}", health),
    }))
}

async fn create_job(
    State(state): State<Arc<ApiState>>,
    Json(config): Json<JobConfig>,
) -> Result<Json<JobResponse>, StatusCode> {
    let mut app_state = state.job_manager.state.lock().await;

    // Update configuration
    app_state.config.webreg.term = config.term.clone();
    app_state.config.webreg.polling_interval = config.polling_interval;
    app_state.config.webreg.cookie = config.cookie.clone();

    // Set seat threshold based on monitoring mode
    app_state.config.monitoring.seat_threshold = match config.monitoring_mode {
        MonitoringMode::Include => 0,  // Any availability
        MonitoringMode::Exclude => config.seat_threshold,  // Custom threshold
    };

    let job_id = Uuid::new_v4().to_string();

    Ok(Json(JobResponse {
        job_id,
        status: "created".to_string(),
        message: "Job configuration saved successfully".to_string(),
    }))
}

async fn start_monitoring(State(state): State<Arc<ApiState>>) -> Result<Json<JobResponse>, StatusCode> {
    match state.job_manager.start().await {
        Ok(_) => Ok(Json(JobResponse {
            job_id: "".to_string(),
            status: "started".to_string(),
            message: "Monitoring started successfully".to_string(),
        })),
        Err(e) => Ok(Json(JobResponse {
            job_id: "".to_string(),
            status: "error".to_string(),
            message: e.to_string(),
        })),
    }
}

async fn stop_monitoring(State(state): State<Arc<ApiState>>) -> Result<Json<JobResponse>, StatusCode> {
    match state.job_manager.stop().await {
        Ok(_) => Ok(Json(JobResponse {
            job_id: "".to_string(),
            status: "stopped".to_string(),
            message: "Monitoring stopped successfully".to_string(),
        })),
        Err(e) => Ok(Json(JobResponse {
            job_id: "".to_string(),
            status: "error".to_string(),
            message: e.to_string(),
        })),
    }
}

async fn get_config(State(state): State<Arc<ApiState>>) -> Result<Json<serde_json::Value>, StatusCode> {
    let app_state = state.job_manager.state.lock().await;

    Ok(Json(serde_json::json!({
        "term": app_state.config.webreg.term,
        "polling_interval": app_state.config.webreg.polling_interval,
        "seat_threshold": app_state.config.monitoring.seat_threshold,
        "monitoring_mode": if app_state.config.monitoring.seat_threshold == 0 { "include" } else { "exclude" }
    })))
}

async fn update_notifications(
    State(state): State<Arc<ApiState>>,
    Json(config): Json<NotificationConfig>,
) -> Result<Json<JobResponse>, StatusCode> {
    let mut app_state = state.job_manager.state.lock().await;

    app_state.config.notifications.gmail_address = config.gmail_address;
    app_state.config.notifications.gmail_app_password = config.gmail_app_password;
    app_state.config.notifications.email_recipients = config.email_recipients;
    app_state.config.notifications.discord_webhook_url = config.discord_webhook_url;

    Ok(Json(JobResponse {
        job_id: "".to_string(),
        status: "updated".to_string(),
        message: "Notification settings updated successfully".to_string(),
    }))
}

// Create router
pub fn create_router(api_state: Arc<ApiState>) -> Router {
    Router::new()
        .route("/api/health", get(health_check))
        .route("/api/status", get(get_status))
        .route("/api/config", get(get_config))
        .route("/api/jobs", post(create_job))
        .route("/api/jobs/start", post(start_monitoring))
        .route("/api/jobs/stop", post(stop_monitoring))
        .route("/api/notifications", post(update_notifications))
        .with_state(api_state)
}
