use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Local};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionFailures {
    pub count: u64,
    pub last_failure: DateTime<Local>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct EnrollmentStats {
    pub total_checks: u64,
    pub openings_found: u64,
    pub enrollment_attempts: u64,
    pub successful_enrollments: u64,
    pub errors: u64,
    pub last_updated: String,
    pub start_time: String,
    pub section_failures: HashMap<String, SectionFailures>,  // Track failures per section
}

impl EnrollmentStats {
    pub fn should_notify_for_section(&mut self, section_id: &str) -> bool {
        let now = Local::now();
        let today = now.date_naive();

        if let Some(failures) = self.section_failures.get(section_id) {
            // Check if the last failure was from a previous day
            if failures.last_failure.date_naive() < today {
                // Reset counter if it's a new day
                self.section_failures.insert(section_id.to_string(), SectionFailures {
                    count: 1,
                    last_failure: now,
                });
                return true;
            }

            // If we're still in the same day, check failure count
            if failures.count >= 3 {
                return false;
            }

            // Increment counter
            self.section_failures.insert(section_id.to_string(), SectionFailures {
                count: failures.count + 1,
                last_failure: now,
            });
            return true;
        }

        // First failure for this section
        self.section_failures.insert(section_id.to_string(), SectionFailures {
            count: 1,
            last_failure: now,
        });
        true
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthStatus {
    pub uptime: String,
    pub last_successful_check: String,
    pub connection_status: bool,
    pub error_count: u64,
    pub success_rate: f64,
    pub total_checks: u64,
}
