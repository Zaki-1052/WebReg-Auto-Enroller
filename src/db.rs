use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use std::error::Error as StdError;
use uuid::Uuid;
use crate::models::*;

pub type DbPool = Pool<Postgres>;

/// Initialize database connection pool
pub async fn init_pool(database_url: &str) -> Result<DbPool, Box<dyn StdError + Send + Sync>> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;

    Ok(pool)
}

// ============================================================================
// User queries
// ============================================================================

/// Get or create user by Clerk user ID
pub async fn get_or_create_user(
    pool: &DbPool,
    clerk_user_id: &str,
    email: &str,
) -> Result<User, Box<dyn StdError + Send + Sync>> {
    // Try to get existing user
    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE clerk_user_id = $1"
    )
    .bind(clerk_user_id)
    .fetch_optional(pool)
    .await?;

    if let Some(user) = user {
        return Ok(user);
    }

    // Create new user
    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (clerk_user_id, email) VALUES ($1, $2) RETURNING *"
    )
    .bind(clerk_user_id)
    .bind(email)
    .fetch_one(pool)
    .await?;

    Ok(user)
}

/// Get user by ID
pub async fn get_user_by_id(
    pool: &DbPool,
    user_id: Uuid,
) -> Result<Option<User>, Box<dyn StdError + Send + Sync>> {
    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE id = $1"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(user)
}

// ============================================================================
// Job queries
// ============================================================================

/// Create a new monitoring job
pub async fn create_job(
    pool: &DbPool,
    user_id: Uuid,
    request: &CreateJobRequest,
    cookie_encrypted: &str,
    encryption_nonce: &str,
) -> Result<Job, Box<dyn StdError + Send + Sync>> {
    let job = sqlx::query_as::<_, Job>(
        r#"
        INSERT INTO jobs (
            user_id, term, polling_interval, cookie_encrypted, encryption_nonce,
            seat_threshold, monitoring_mode
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#
    )
    .bind(user_id)
    .bind(&request.term)
    .bind(request.polling_interval)
    .bind(cookie_encrypted)
    .bind(encryption_nonce)
    .bind(request.seat_threshold)
    .bind(&request.monitoring_mode)
    .fetch_one(pool)
    .await?;

    Ok(job)
}

/// Get all jobs for a user
pub async fn get_user_jobs(
    pool: &DbPool,
    user_id: Uuid,
) -> Result<Vec<Job>, Box<dyn StdError + Send + Sync>> {
    let jobs = sqlx::query_as::<_, Job>(
        "SELECT * FROM jobs WHERE user_id = $1 ORDER BY created_at DESC"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(jobs)
}

/// Get a specific job by ID (with user ownership check)
pub async fn get_job_by_id(
    pool: &DbPool,
    job_id: Uuid,
    user_id: Uuid,
) -> Result<Option<Job>, Box<dyn StdError + Send + Sync>> {
    let job = sqlx::query_as::<_, Job>(
        "SELECT * FROM jobs WHERE id = $1 AND user_id = $2"
    )
    .bind(job_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(job)
}

/// Update job status
pub async fn update_job_status(
    pool: &DbPool,
    job_id: Uuid,
    is_active: bool,
    is_connected: bool,
) -> Result<(), Box<dyn StdError + Send + Sync>> {
    sqlx::query(
        "UPDATE jobs SET is_active = $1, is_connected = $2, updated_at = NOW() WHERE id = $3"
    )
    .bind(is_active)
    .bind(is_connected)
    .bind(job_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Update job last check time
pub async fn update_job_last_check(
    pool: &DbPool,
    job_id: Uuid,
) -> Result<(), Box<dyn StdError + Send + Sync>> {
    sqlx::query(
        "UPDATE jobs SET last_check_time = NOW(), updated_at = NOW() WHERE id = $1"
    )
    .bind(job_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Delete a job (cascades to courses, sections, stats)
pub async fn delete_job(
    pool: &DbPool,
    job_id: Uuid,
    user_id: Uuid,
) -> Result<bool, Box<dyn StdError + Send + Sync>> {
    let result = sqlx::query(
        "DELETE FROM jobs WHERE id = $1 AND user_id = $2"
    )
    .bind(job_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Get all active jobs (for server to monitor)
pub async fn get_all_active_jobs(
    pool: &DbPool,
) -> Result<Vec<Job>, Box<dyn StdError + Send + Sync>> {
    let jobs = sqlx::query_as::<_, Job>(
        "SELECT * FROM jobs WHERE is_active = true"
    )
    .fetch_all(pool)
    .await?;

    Ok(jobs)
}

// ============================================================================
// Course queries
// ============================================================================

/// Create courses for a job
pub async fn create_courses(
    pool: &DbPool,
    job_id: Uuid,
    courses: &[CourseRequest],
) -> Result<Vec<Course>, Box<dyn StdError + Send + Sync>> {
    let mut created_courses = Vec::new();

    for course_req in courses {
        let course = sqlx::query_as::<_, Course>(
            "INSERT INTO courses (job_id, department, course_code) VALUES ($1, $2, $3) RETURNING *"
        )
        .bind(job_id)
        .bind(&course_req.department)
        .bind(&course_req.course_code)
        .fetch_one(pool)
        .await?;

        created_courses.push(course);
    }

    Ok(created_courses)
}

/// Get courses for a job
pub async fn get_job_courses(
    pool: &DbPool,
    job_id: Uuid,
) -> Result<Vec<Course>, Box<dyn StdError + Send + Sync>> {
    let courses = sqlx::query_as::<_, Course>(
        "SELECT * FROM courses WHERE job_id = $1"
    )
    .bind(job_id)
    .fetch_all(pool)
    .await?;

    Ok(courses)
}

// ============================================================================
// Section queries
// ============================================================================

/// Create sections for a course
pub async fn create_sections(
    pool: &DbPool,
    course_id: Uuid,
    sections: &[SectionRequest],
) -> Result<Vec<Section>, Box<dyn StdError + Send + Sync>> {
    let mut created_sections = Vec::new();

    for section_req in sections {
        let discussions_json = serde_json::to_value(&section_req.discussions)?;

        let section = sqlx::query_as::<_, Section>(
            "INSERT INTO sections (course_id, lecture, discussions) VALUES ($1, $2, $3) RETURNING *"
        )
        .bind(course_id)
        .bind(&section_req.lecture)
        .bind(discussions_json)
        .fetch_one(pool)
        .await?;

        created_sections.push(section);
    }

    Ok(created_sections)
}

/// Get sections for a course
pub async fn get_course_sections(
    pool: &DbPool,
    course_id: Uuid,
) -> Result<Vec<Section>, Box<dyn StdError + Send + Sync>> {
    let sections = sqlx::query_as::<_, Section>(
        "SELECT * FROM sections WHERE course_id = $1"
    )
    .bind(course_id)
    .fetch_all(pool)
    .await?;

    Ok(sections)
}

// ============================================================================
// Stats queries
// ============================================================================

/// Initialize stats for a job
pub async fn init_job_stats(
    pool: &DbPool,
    job_id: Uuid,
) -> Result<EnrollmentStatsDb, Box<dyn StdError + Send + Sync>> {
    let stats = sqlx::query_as::<_, EnrollmentStatsDb>(
        "INSERT INTO enrollment_stats (job_id) VALUES ($1) RETURNING *"
    )
    .bind(job_id)
    .fetch_one(pool)
    .await?;

    Ok(stats)
}

/// Get stats for a job
pub async fn get_job_stats(
    pool: &DbPool,
    job_id: Uuid,
) -> Result<Option<EnrollmentStatsDb>, Box<dyn StdError + Send + Sync>> {
    let stats = sqlx::query_as::<_, EnrollmentStatsDb>(
        "SELECT * FROM enrollment_stats WHERE job_id = $1"
    )
    .bind(job_id)
    .fetch_optional(pool)
    .await?;

    Ok(stats)
}

/// Update stats for a job
pub async fn update_job_stats(
    pool: &DbPool,
    job_id: Uuid,
    total_checks: i32,
    openings_found: i32,
    enrollment_attempts: i32,
    successful_enrollments: i32,
    errors: i32,
    section_failures: serde_json::Value,
) -> Result<(), Box<dyn StdError + Send + Sync>> {
    sqlx::query(
        r#"
        UPDATE enrollment_stats SET
            total_checks = $1,
            openings_found = $2,
            enrollment_attempts = $3,
            successful_enrollments = $4,
            errors = $5,
            section_failures = $6,
            last_updated = NOW()
        WHERE job_id = $7
        "#
    )
    .bind(total_checks)
    .bind(openings_found)
    .bind(enrollment_attempts)
    .bind(successful_enrollments)
    .bind(errors)
    .bind(section_failures)
    .bind(job_id)
    .execute(pool)
    .await?;

    Ok(())
}

// ============================================================================
// Notification queries
// ============================================================================

/// Get or create notification settings for a user
pub async fn get_or_create_notification_settings(
    pool: &DbPool,
    user_id: Uuid,
) -> Result<NotificationSettings, Box<dyn StdError + Send + Sync>> {
    // Try to get existing settings
    let settings = sqlx::query_as::<_, NotificationSettings>(
        "SELECT * FROM notification_settings WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    if let Some(settings) = settings {
        return Ok(settings);
    }

    // Create new settings with empty values
    let settings = sqlx::query_as::<_, NotificationSettings>(
        "INSERT INTO notification_settings (user_id) VALUES ($1) RETURNING *"
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(settings)
}

/// Update notification settings
pub async fn update_notification_settings(
    pool: &DbPool,
    user_id: Uuid,
    gmail_address: Option<&str>,
    gmail_encrypted: Option<&str>,
    gmail_nonce: Option<&str>,
    email_recipients: &[String],
    discord_webhook: Option<&str>,
) -> Result<(), Box<dyn StdError + Send + Sync>> {
    let recipients_json = serde_json::to_value(email_recipients)?;

    sqlx::query(
        r#"
        UPDATE notification_settings SET
            gmail_address = $1,
            gmail_app_password_encrypted = $2,
            gmail_encryption_nonce = $3,
            email_recipients = $4,
            discord_webhook_url = $5,
            updated_at = NOW()
        WHERE user_id = $6
        "#
    )
    .bind(gmail_address)
    .bind(gmail_encrypted)
    .bind(gmail_nonce)
    .bind(recipients_json)
    .bind(discord_webhook)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}
