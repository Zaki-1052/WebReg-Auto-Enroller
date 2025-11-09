use std::error::Error as StdError;
use webweg::wrapper::{WebRegWrapper, input_types::{AddType, EnrollWaitAdd, GradeOption}};
use chrono::Local;
use log::{info, warn, error};
use crate::notifier::Notifier;
use crate::stats::EnrollmentStats;
use crate::utils::get_retry_strategy;

pub async fn try_enroll(
    wrapper: &WebRegWrapper,
    term: &str,
    section_id: &str,
) -> Result<bool, Box<dyn StdError + Send + Sync>> {
    let enroll_request = EnrollWaitAdd::builder()
        .with_section_id(section_id)
        .with_grading_option(GradeOption::L)
        .try_build()
        .ok_or("Failed to build enrollment request")?;

    match wrapper.req(term).parsed().add_section(AddType::Enroll, enroll_request, true).await {
        Ok(result) => {
            info!("Enrollment attempt result: {:?}", result);
            Ok(result)
        },
        Err(e) => {
            error!("Enrollment error: {:?}", e);
            Ok(false)
        }
    }
}

pub async fn try_enroll_with_retry(
    wrapper: &WebRegWrapper,
    term: &str,
    section_id: &str,
    department: &str,
    course_code: &str,
    section: &str,
    notifier: &Notifier,
    stats: &mut EnrollmentStats,
) -> Result<bool, Box<dyn StdError + Send + Sync>> {
    let retry_strategy = get_retry_strategy();

    let result = tokio_retry::Retry::spawn(retry_strategy, || async {
        match try_enroll(wrapper, term, section_id).await {
            Ok(result) => Ok(result),
            Err(e) => {
                warn!("Enrollment error: {:?}, retrying...", e);
                Err(e)
            }
        }
    }).await?;

    let section_key = format!("{}_{}_{}_{}", department, course_code, section, term);

    if result {
        // On success, remove any failure tracking for this section
        stats.section_failures.remove(&section_key);

        let msg = format!(
            "Successfully enrolled in {} {} section {}!\n\nTime: {}\nPlease verify on WebReg.",
            department, course_code, section, Local::now().format("%Y-%m-%d %H:%M:%S")
        );
        notifier.send_notification(&msg).await;
    } else {
        // Check if we should notify for this section
        if stats.should_notify_for_section(&section_key) {
            let msg = format!(
                "Failed to enroll in {} {} section {} despite available seats.\n\nTime: {}\nPlease check WebReg manually.",
                department, course_code, section, Local::now().format("%Y-%m-%d %H:%M:%S")
            );
            notifier.send_notification(&msg).await;
        } else {
            info!("Suppressing notification for {} {} section {} (exceeded daily failure limit)",
                department, course_code, section);
        }
    }

    Ok(result)
}
