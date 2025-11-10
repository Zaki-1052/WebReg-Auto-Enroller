use serde::{Deserialize, Serialize};

// Constants
pub const DEFAULT_RETRY_ATTEMPTS: u32 = 3;
pub const DEFAULT_RETRY_DELAY: u64 = 1000;
pub const CONFIG_PATH: &str = "config.toml";

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub webreg: WebRegConfig,
    pub notifications: NotificationConfig,
    pub courses: CourseConfig,
    pub monitoring: MonitoringConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WebRegConfig {
    pub term: String,
    pub polling_interval: u64,
    pub cookie: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct NotificationConfig {
    pub gmail_address: String,
    pub gmail_app_password: String,
    pub email_recipients: Vec<String>,
    pub discord_webhook_url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CourseConfig {
    pub chem: CourseDetails,
    pub bild: LegacyCourseDetails,  // Use old format for BILD
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]  // This allows serde to try both formats
pub enum CourseDetails {
    New(NewCourseDetails),
    Legacy(LegacyCourseDetails),
}

#[derive(Debug, Deserialize, Clone)]
pub struct NewCourseDetails {
    pub department: String,
    pub course_code: String,
    pub sections: Vec<SectionGroup>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LegacyCourseDetails {
    pub department: String,
    pub course_code: String,
    pub lecture_section: String,
    pub discussion_sections: Vec<String>,
}

impl CourseDetails {
    pub fn department(&self) -> &str {
        match self {
            CourseDetails::New(details) => &details.department,
            CourseDetails::Legacy(details) => &details.department,
        }
    }

    pub fn course_code(&self) -> &str {
        match self {
            CourseDetails::New(details) => &details.course_code,
            CourseDetails::Legacy(details) => &details.course_code,
        }
    }
}

pub fn to_section_groups(course: &LegacyCourseDetails) -> Vec<SectionGroup> {
    vec![SectionGroup {
        lecture: course.lecture_section.clone(),
        discussions: course.discussion_sections.clone(),
    }]
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SectionGroup {
    pub lecture: String,
    pub discussions: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MonitoringConfig {
    pub log_file: String,
    pub stats_file: String,
    pub cookie_refresh_interval: u64,
    pub max_retries: u32,
    pub retry_delay: u64,
    #[serde(default = "default_seat_threshold")]
    pub seat_threshold: i64,  // Threshold for available seats (0 = any availability, 3 = fewer than 3 seats)
}

fn default_seat_threshold() -> i64 {
    0  // Default to aggressive mode (any seat availability)
}
