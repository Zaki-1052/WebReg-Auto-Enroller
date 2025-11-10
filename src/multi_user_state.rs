use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use std::time::{Duration, SystemTime};
use tokio::time::sleep;
use log::{info, error};
use chrono::Local;
use uuid::Uuid;
use webweg::wrapper::WebRegWrapper;

use crate::db::DbPool;
use crate::models::*;
use crate::encryption::EncryptionKey;
use crate::notifier::Notifier;
use crate::stats::EnrollmentStats;
use crate::monitor::monitor_section_with_retry;
use crate::enroll::try_enroll_with_retry;

/// Represents a running monitoring job for a user
pub struct UserJob {
    pub job_id: Uuid,
    pub user_id: Uuid,
    pub term: String,
    pub wrapper: Arc<WebRegWrapper>,
    pub notifier: Notifier,
    pub stats: EnrollmentStats,
    pub courses: Vec<CourseWithSections>,
    pub polling_interval: u64,
    pub seat_threshold: i64,
    pub is_running: bool,
    pub is_connected: bool,
    pub last_check_time: String,
    pub start_time: SystemTime,
    pub shutdown_tx: tokio::sync::broadcast::Sender<()>,
}

#[derive(Clone)]
pub struct CourseWithSections {
    pub department: String,
    pub course_code: String,
    pub sections: Vec<SectionGroup>,
}

#[derive(Clone)]
pub struct SectionGroup {
    pub lecture: String,
    pub discussions: Vec<String>,
}

/// Global state managing all user jobs
pub struct MultiUserState {
    pub pool: DbPool,
    pub encryption_key: EncryptionKey,
    pub jobs: Arc<RwLock<HashMap<Uuid, Arc<Mutex<UserJob>>>>>,
}

impl MultiUserState {
    pub fn new(pool: DbPool, encryption_key: EncryptionKey) -> Self {
        Self {
            pool,
            encryption_key,
            jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new job for a user
    pub async fn create_job(
        &self,
        user_id: Uuid,
        request: CreateJobRequest,
    ) -> Result<Uuid, Box<dyn std::error::Error + Send + Sync>> {
        // Encrypt the cookie
        let (cookie_encrypted, encryption_nonce) = self.encryption_key.encrypt(&request.cookie)?;

        // Create job in database
        let job = crate::db::create_job(
            &self.pool,
            user_id,
            &request,
            &cookie_encrypted,
            &encryption_nonce,
        )
        .await?;

        // Create courses in database
        let courses = crate::db::create_courses(&self.pool, job.id, &request.courses).await?;

        // Create sections for each course
        for (i, course) in courses.iter().enumerate() {
            crate::db::create_sections(&self.pool, course.id, &request.courses[i].sections).await?;
        }

        // Initialize stats
        crate::db::init_job_stats(&self.pool, job.id).await?;

        Ok(job.id)
    }

    /// Start a job for a user
    pub async fn start_job(&self, job_id: Uuid, user_id: Uuid) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get job from database
        let job = crate::db::get_job_by_id(&self.pool, job_id, user_id)
            .await?
            .ok_or("Job not found")?;

        // Check if job is already running
        let jobs_read = self.jobs.read().await;
        if jobs_read.contains_key(&job_id) {
            return Err("Job is already running".into());
        }
        drop(jobs_read);

        // Decrypt cookie
        let cookie = self.encryption_key.decrypt(&job.cookie_encrypted, &job.encryption_nonce)?;

        // Create WebReg wrapper
        let wrapper = WebRegWrapper::builder()
            .with_cookies(&cookie)
            .try_build_wrapper()
            .ok_or("Failed to create WebRegWrapper")?;

        // Get courses and sections
        let courses = crate::db::get_job_courses(&self.pool, job_id).await?;
        let mut course_sections = Vec::new();

        for course in courses {
            let sections = crate::db::get_course_sections(&self.pool, course.id).await?;
            let section_groups: Vec<SectionGroup> = sections
                .iter()
                .map(|s| {
                    let discussions: Vec<String> = serde_json::from_value(s.discussions.clone())
                        .unwrap_or_default();
                    SectionGroup {
                        lecture: s.lecture.clone(),
                        discussions,
                    }
                })
                .collect();

            course_sections.push(CourseWithSections {
                department: course.department,
                course_code: course.course_code,
                sections: section_groups,
            });
        }

        // Get notification settings
        let notification_settings = crate::db::get_or_create_notification_settings(&self.pool, user_id).await?;

        // Decrypt gmail password if present
        let gmail_password = if let (Some(encrypted), Some(nonce)) = (
            &notification_settings.gmail_app_password_encrypted,
            &notification_settings.gmail_encryption_nonce,
        ) {
            Some(self.encryption_key.decrypt(encrypted, nonce)?)
        } else {
            None
        };

        // Create notifier configuration
        let email_recipients: Vec<String> = serde_json::from_value(notification_settings.email_recipients.clone())
            .unwrap_or_default();

        let notification_config = crate::config::NotificationConfig {
            gmail_address: notification_settings.gmail_address.clone().unwrap_or_default(),
            gmail_app_password: gmail_password.unwrap_or_default(),
            email_recipients,
            discord_webhook_url: notification_settings.discord_webhook_url.clone().unwrap_or_default(),
        };

        let notifier = Notifier::new(&notification_config)?;

        // Get or initialize stats
        let stats_db = crate::db::get_job_stats(&self.pool, job_id).await?
            .ok_or("Stats not found")?;

        let stats = EnrollmentStats {
            start_time: stats_db.start_time.to_string(),
            last_updated: stats_db.last_updated.to_string(),
            total_checks: stats_db.total_checks as u64,
            openings_found: stats_db.openings_found as u64,
            enrollment_attempts: stats_db.enrollment_attempts as u64,
            successful_enrollments: stats_db.successful_enrollments as u64,
            errors: stats_db.errors as u64,
            section_failures: serde_json::from_value(stats_db.section_failures).unwrap_or_default(),
        };

        // Create shutdown channel
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);

        // Create user job
        let user_job = Arc::new(Mutex::new(UserJob {
            job_id,
            user_id,
            term: job.term.clone(),
            wrapper: Arc::new(wrapper),
            notifier,
            stats,
            courses: course_sections,
            polling_interval: job.polling_interval as u64,
            seat_threshold: job.seat_threshold as i64,
            is_running: true,
            is_connected: true,
            last_check_time: Local::now().to_string(),
            start_time: SystemTime::now(),
            shutdown_tx: shutdown_tx.clone(),
        }));

        // Add to jobs map
        let mut jobs_write = self.jobs.write().await;
        jobs_write.insert(job_id, user_job.clone());
        drop(jobs_write);

        // Update job status in database
        crate::db::update_job_status(&self.pool, job_id, true, true).await?;

        // Spawn monitoring task
        let pool_clone = self.pool.clone();
        tokio::spawn(async move {
            Self::run_monitoring_loop(user_job, pool_clone).await;
        });

        Ok(())
    }

    /// Stop a job
    pub async fn stop_job(&self, job_id: Uuid) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let jobs_read = self.jobs.read().await;
        let job = jobs_read.get(&job_id).ok_or("Job not running")?;

        let job_lock = job.lock().await;
        let _ = job_lock.shutdown_tx.send(());
        drop(job_lock);
        drop(jobs_read);

        // Remove from jobs map
        let mut jobs_write = self.jobs.write().await;
        jobs_write.remove(&job_id);
        drop(jobs_write);

        // Update database
        crate::db::update_job_status(&self.pool, job_id, false, false).await?;

        Ok(())
    }

    /// Get job status
    pub async fn get_job_status(&self, job_id: Uuid) -> Option<JobStatusInfo> {
        let jobs_read = self.jobs.read().await;
        let job = jobs_read.get(&job_id)?;
        let job_lock = job.lock().await;

        Some(JobStatusInfo {
            is_running: job_lock.is_running,
            is_connected: job_lock.is_connected,
            last_check_time: job_lock.last_check_time.clone(),
            stats: job_lock.stats.clone(),
        })
    }

    /// Monitoring loop for a user job
    async fn run_monitoring_loop(job: Arc<Mutex<UserJob>>, pool: DbPool) {
        let mut shutdown_rx = {
            let job_lock = job.lock().await;
            job_lock.shutdown_tx.subscribe()
        };

        let cookie_refresh_interval = 480; // 8 minutes
        let mut cookie_refresh_timer = tokio::time::interval(Duration::from_secs(cookie_refresh_interval));

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    info!("Received shutdown signal for job");
                    let mut job_lock = job.lock().await;
                    job_lock.is_running = false;
                    break;
                }
                _ = cookie_refresh_timer.tick() => {
                    // TODO: Implement cookie refresh logic
                    info!("Cookie refresh tick");
                }
                _ = async {
                    let mut job_lock = job.lock().await;

                    if !job_lock.is_running || !job_lock.is_connected {
                        let polling_interval = job_lock.polling_interval;
                        drop(job_lock);
                        sleep(Duration::from_secs(polling_interval)).await;
                        return;
                    }

                    // Get necessary data for monitoring (clone to avoid borrow checker issues)
                    let job_id = job_lock.job_id;
                    let term = job_lock.term.clone();
                    let wrapper = Arc::clone(&job_lock.wrapper);
                    let notifier = job_lock.notifier.clone();
                    let courses = job_lock.courses.clone();
                    let polling_interval = job_lock.polling_interval;
                    let seat_threshold = job_lock.seat_threshold;

                    // Monitor each course
                    for course in &courses {
                        for section_group in &course.sections {
                            // Monitor lecture
                            if let Ok(Some(section_id)) = monitor_section_with_retry(
                                &wrapper,
                                &term,
                                &section_group.lecture,
                                &course.department,
                                &course.course_code,
                                polling_interval,
                                seat_threshold,
                                &notifier,
                            ).await {
                                job_lock.stats.enrollment_attempts += 1;

                                if let Ok(true) = try_enroll_with_retry(
                                    &wrapper,
                                    &term,
                                    &section_id,
                                    &course.department,
                                    &course.course_code,
                                    &section_group.lecture,
                                    &notifier,
                                    &mut job_lock.stats,
                                ).await {
                                    job_lock.stats.successful_enrollments += 1;
                                }
                            }

                            // Monitor discussions
                            for discussion in &section_group.discussions {
                                if let Ok(Some(section_id)) = monitor_section_with_retry(
                                    &wrapper,
                                    &term,
                                    discussion,
                                    &course.department,
                                    &course.course_code,
                                    polling_interval,
                                    seat_threshold,
                                    &notifier,
                                ).await {
                                    job_lock.stats.enrollment_attempts += 1;

                                    if let Ok(true) = try_enroll_with_retry(
                                        &wrapper,
                                        &term,
                                        &section_id,
                                        &course.department,
                                        &course.course_code,
                                        discussion,
                                        &notifier,
                                        &mut job_lock.stats,
                                    ).await {
                                        job_lock.stats.successful_enrollments += 1;
                                    }
                                }
                            }
                        }
                    }

                    job_lock.last_check_time = Local::now().to_string();
                    job_lock.stats.total_checks += 1;

                    // Update stats in database
                    let stats_json = serde_json::to_value(&job_lock.stats.section_failures).unwrap_or_default();
                    let _ = crate::db::update_job_stats(
                        &pool,
                        job_id,
                        job_lock.stats.total_checks as i32,
                        job_lock.stats.openings_found as i32,
                        job_lock.stats.enrollment_attempts as i32,
                        job_lock.stats.successful_enrollments as i32,
                        job_lock.stats.errors as i32,
                        stats_json,
                    ).await;

                    let _ = crate::db::update_job_last_check(&pool, job_id).await;

                    drop(job_lock);
                    sleep(Duration::from_secs(polling_interval)).await;
                } => {}
            }
        }
    }

    /// Get all user jobs (from database, not just running ones)
    pub async fn get_user_jobs(&self, user_id: Uuid) -> Result<Vec<Job>, Box<dyn std::error::Error + Send + Sync>> {
        crate::db::get_user_jobs(&self.pool, user_id).await
    }

    /// Delete a user job
    pub async fn delete_job(&self, job_id: Uuid, user_id: Uuid) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Stop if running
        if self.jobs.read().await.contains_key(&job_id) {
            self.stop_job(job_id).await?;
        }

        // Delete from database
        crate::db::delete_job(&self.pool, job_id, user_id).await?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct JobStatusInfo {
    pub is_running: bool,
    pub is_connected: bool,
    pub last_check_time: String,
    pub stats: EnrollmentStats,
}
