use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub clerk_user_id: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Job {
    pub id: Uuid,
    pub user_id: Uuid,
    pub term: String,
    pub polling_interval: i32,
    pub cookie_encrypted: String,
    pub encryption_nonce: String,
    pub seat_threshold: i32,
    pub monitoring_mode: String,
    pub is_active: bool,
    pub is_connected: bool,
    pub last_check_time: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Course {
    pub id: Uuid,
    pub job_id: Uuid,
    pub department: String,
    pub course_code: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Section {
    pub id: Uuid,
    pub course_id: Uuid,
    pub lecture: String,
    pub discussions: sqlx::types::JsonValue,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EnrollmentStatsDb {
    pub id: Uuid,
    pub job_id: Uuid,
    pub total_checks: i32,
    pub openings_found: i32,
    pub enrollment_attempts: i32,
    pub successful_enrollments: i32,
    pub errors: i32,
    pub section_failures: sqlx::types::JsonValue,
    pub start_time: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NotificationSettings {
    pub id: Uuid,
    pub user_id: Uuid,
    pub gmail_address: Option<String>,
    pub gmail_app_password_encrypted: Option<String>,
    pub gmail_encryption_nonce: Option<String>,
    pub email_recipients: sqlx::types::JsonValue,
    pub discord_webhook_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Request/Response DTOs
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateJobRequest {
    pub term: String,
    pub polling_interval: i32,
    pub cookie: String,
    pub seat_threshold: i32,
    pub monitoring_mode: String,
    pub courses: Vec<CourseRequest>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CourseRequest {
    pub department: String,
    pub course_code: String,
    pub sections: Vec<SectionRequest>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SectionRequest {
    pub lecture: String,
    pub discussions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateNotificationRequest {
    pub gmail_address: Option<String>,
    pub gmail_app_password: Option<String>,
    pub email_recipients: Vec<String>,
    pub discord_webhook_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JobResponse {
    pub id: Uuid,
    pub term: String,
    pub polling_interval: i32,
    pub seat_threshold: i32,
    pub monitoring_mode: String,
    pub is_active: bool,
    pub is_connected: bool,
    pub last_check_time: Option<DateTime<Utc>>,
    pub courses: Vec<CourseResponse>,
    pub stats: Option<EnrollmentStatsResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CourseResponse {
    pub id: Uuid,
    pub department: String,
    pub course_code: String,
    pub sections: Vec<SectionResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SectionResponse {
    pub id: Uuid,
    pub lecture: String,
    pub discussions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnrollmentStatsResponse {
    pub total_checks: i32,
    pub openings_found: i32,
    pub enrollment_attempts: i32,
    pub successful_enrollments: i32,
    pub errors: i32,
    pub section_failures: serde_json::Value,
    pub start_time: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}
