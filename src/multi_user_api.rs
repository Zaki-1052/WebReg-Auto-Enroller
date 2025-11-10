use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthenticatedUser;
use crate::multi_user_state::MultiUserState;
use crate::models::*;
use crate::db;

// ============================================================================
// API State
// ============================================================================

pub struct MultiUserApiState {
    pub state: Arc<MultiUserState>,
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct JobListItem {
    pub id: Uuid,
    pub term: String,
    pub polling_interval: i32,
    pub seat_threshold: i32,
    pub monitoring_mode: String,
    pub is_active: bool,
    pub is_connected: bool,
    pub last_check_time: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct JobDetailResponse {
    pub job: JobResponse,
    pub is_running: bool,
}

// ============================================================================
// API Handlers
// ============================================================================

/// Health check endpoint
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "WebReg Auto-Enroller Multi-User",
        "version": "2.0.0"
    }))
}

/// Get current user profile
async fn get_current_user(
    State(state): State<Arc<MultiUserApiState>>,
    auth: AuthenticatedUser,
) -> Result<Json<ApiResponse<User>>, StatusCode> {
    let user = db::get_or_create_user(&state.state.pool, &auth.clerk_user_id, &auth.email)
        .await
        .map_err(|e| {
            log::error!("Failed to get user: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(ApiResponse::success(user)))
}

/// Create a new monitoring job
async fn create_job(
    State(state): State<Arc<MultiUserApiState>>,
    auth: AuthenticatedUser,
    Json(request): Json<CreateJobRequest>,
) -> Result<Json<ApiResponse<Uuid>>, StatusCode> {
    // Get or create user
    let user = db::get_or_create_user(&state.state.pool, &auth.clerk_user_id, &auth.email)
        .await
        .map_err(|e| {
            log::error!("Failed to get user: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Create job
    let job_id = state.state.create_job(user.id, request)
        .await
        .map_err(|e| {
            log::error!("Failed to create job: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(ApiResponse::success(job_id)))
}

/// Get all jobs for the current user
async fn get_user_jobs(
    State(state): State<Arc<MultiUserApiState>>,
    auth: AuthenticatedUser,
) -> Result<Json<ApiResponse<Vec<JobListItem>>>, StatusCode> {
    let user = db::get_or_create_user(&state.state.pool, &auth.clerk_user_id, &auth.email)
        .await
        .map_err(|e| {
            log::error!("Failed to get user: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let jobs = state.state.get_user_jobs(user.id)
        .await
        .map_err(|e| {
            log::error!("Failed to get jobs: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let job_items: Vec<JobListItem> = jobs.iter().map(|j| JobListItem {
        id: j.id,
        term: j.term.clone(),
        polling_interval: j.polling_interval,
        seat_threshold: j.seat_threshold,
        monitoring_mode: j.monitoring_mode.clone(),
        is_active: j.is_active,
        is_connected: j.is_connected,
        last_check_time: j.last_check_time.map(|t| t.to_string()),
        created_at: j.created_at.to_string(),
    }).collect();

    Ok(Json(ApiResponse::success(job_items)))
}

/// Get a specific job with details
async fn get_job_detail(
    State(state): State<Arc<MultiUserApiState>>,
    auth: AuthenticatedUser,
    Path(job_id): Path<Uuid>,
) -> Result<Json<ApiResponse<JobDetailResponse>>, StatusCode> {
    let user = db::get_or_create_user(&state.state.pool, &auth.clerk_user_id, &auth.email)
        .await
        .map_err(|e| {
            log::error!("Failed to get user: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let job = db::get_job_by_id(&state.state.pool, job_id, user.id)
        .await
        .map_err(|e| {
            log::error!("Failed to get job: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Get courses
    let courses = db::get_job_courses(&state.state.pool, job_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut course_responses = Vec::new();
    for course in courses {
        let sections = db::get_course_sections(&state.state.pool, course.id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let section_responses: Vec<SectionResponse> = sections.iter().map(|s| {
            let discussions: Vec<String> = serde_json::from_value(s.discussions.clone())
                .unwrap_or_default();
            SectionResponse {
                id: s.id,
                lecture: s.lecture.clone(),
                discussions,
            }
        }).collect();

        course_responses.push(CourseResponse {
            id: course.id,
            department: course.department,
            course_code: course.course_code,
            sections: section_responses,
        });
    }

    // Get stats
    let stats_db = db::get_job_stats(&state.state.pool, job_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let stats = stats_db.map(|s| EnrollmentStatsResponse {
        total_checks: s.total_checks,
        openings_found: s.openings_found,
        enrollment_attempts: s.enrollment_attempts,
        successful_enrollments: s.successful_enrollments,
        errors: s.errors,
        section_failures: s.section_failures,
        start_time: s.start_time,
        last_updated: s.last_updated,
    });

    // Check if job is currently running
    let is_running = state.state.get_job_status(job_id).await.is_some();

    let job_response = JobResponse {
        id: job.id,
        term: job.term,
        polling_interval: job.polling_interval,
        seat_threshold: job.seat_threshold,
        monitoring_mode: job.monitoring_mode,
        is_active: job.is_active,
        is_connected: job.is_connected,
        last_check_time: job.last_check_time,
        courses: course_responses,
        stats,
    };

    Ok(Json(ApiResponse::success(JobDetailResponse {
        job: job_response,
        is_running,
    })))
}

/// Start a job
async fn start_job(
    State(state): State<Arc<MultiUserApiState>>,
    auth: AuthenticatedUser,
    Path(job_id): Path<Uuid>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let user = db::get_or_create_user(&state.state.pool, &auth.clerk_user_id, &auth.email)
        .await
        .map_err(|e| {
            log::error!("Failed to get user: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    state.state.start_job(job_id, user.id)
        .await
        .map_err(|e| {
            log::error!("Failed to start job: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(ApiResponse::success("Job started successfully".to_string())))
}

/// Stop a job
async fn stop_job(
    State(state): State<Arc<MultiUserApiState>>,
    auth: AuthenticatedUser,
    Path(job_id): Path<Uuid>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let user = db::get_or_create_user(&state.state.pool, &auth.clerk_user_id, &auth.email)
        .await
        .map_err(|e| {
            log::error!("Failed to get user: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Verify ownership
    let _job = db::get_job_by_id(&state.state.pool, job_id, user.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    state.state.stop_job(job_id)
        .await
        .map_err(|e| {
            log::error!("Failed to stop job: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(ApiResponse::success("Job stopped successfully".to_string())))
}

/// Delete a job
async fn delete_job(
    State(state): State<Arc<MultiUserApiState>>,
    auth: AuthenticatedUser,
    Path(job_id): Path<Uuid>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let user = db::get_or_create_user(&state.state.pool, &auth.clerk_user_id, &auth.email)
        .await
        .map_err(|e| {
            log::error!("Failed to get user: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    state.state.delete_job(job_id, user.id)
        .await
        .map_err(|e| {
            log::error!("Failed to delete job: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(ApiResponse::success("Job deleted successfully".to_string())))
}

/// Get notification settings
async fn get_notifications(
    State(state): State<Arc<MultiUserApiState>>,
    auth: AuthenticatedUser,
) -> Result<Json<ApiResponse<NotificationSettings>>, StatusCode> {
    let user = db::get_or_create_user(&state.state.pool, &auth.clerk_user_id, &auth.email)
        .await
        .map_err(|e| {
            log::error!("Failed to get user: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let settings = db::get_or_create_notification_settings(&state.state.pool, user.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::success(settings)))
}

/// Update notification settings
async fn update_notifications(
    State(state): State<Arc<MultiUserApiState>>,
    auth: AuthenticatedUser,
    Json(request): Json<UpdateNotificationRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let user = db::get_or_create_user(&state.state.pool, &auth.clerk_user_id, &auth.email)
        .await
        .map_err(|e| {
            log::error!("Failed to get user: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Encrypt gmail password if provided
    let (gmail_encrypted, gmail_nonce) = if let Some(password) = &request.gmail_app_password {
        let (enc, nonce) = state.state.encryption_key.encrypt(password)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        (Some(enc), Some(nonce))
    } else {
        (None, None)
    };

    db::update_notification_settings(
        &state.state.pool,
        user.id,
        request.gmail_address.as_deref(),
        gmail_encrypted.as_deref(),
        gmail_nonce.as_deref(),
        &request.email_recipients,
        request.discord_webhook_url.as_deref(),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::success("Notifications updated successfully".to_string())))
}

// ============================================================================
// Router
// ============================================================================

pub fn create_router(state: Arc<MultiUserApiState>) -> Router {
    Router::new()
        // Public routes
        .route("/api/health", get(health_check))

        // Authenticated routes
        .route("/api/user", get(get_current_user))
        .route("/api/jobs", post(create_job))
        .route("/api/jobs", get(get_user_jobs))
        .route("/api/jobs/:job_id", get(get_job_detail))
        .route("/api/jobs/:job_id/start", post(start_job))
        .route("/api/jobs/:job_id/stop", post(stop_job))
        .route("/api/jobs/:job_id", delete(delete_job))
        .route("/api/notifications", get(get_notifications))
        .route("/api/notifications", post(update_notifications))

        .with_state(state)
}
