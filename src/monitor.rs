use std::error::Error as StdError;
use std::fs::OpenOptions;
use std::io::Write;
use webweg::wrapper::WebRegWrapper;
use chrono::Local;
use log::{info, warn};
use crate::notifier::Notifier;
use crate::utils::get_retry_strategy;

pub async fn monitor_section(
    wrapper: &WebRegWrapper,
    term: &str,
    section: &str,
    department: &str,
    course_code: &str,
    polling_interval: u64,
    seat_threshold: i64,
) -> Result<Option<String>, Box<dyn StdError + Send + Sync>> {
    let course_info = wrapper.req(term).parsed().get_course_info(department, course_code).await?;

    for section_info in course_info {
        if section_info.section_code == section {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S.%f").to_string();
            let details = format!(
                "[{}] {} {} Section {} Details:\n\
                Section ID: {}\n\
                Section Code: {}\n\
                Available Seats: {}\n\
                Total Seats: {}\n\
                Enrolled Count: {}\n\
                Waitlist Count: {}\n\
                Raw API Response: {:#?}\n\
                -------------------\n",
                timestamp,
                department,
                course_code,
                section,
                section_info.section_id,
                section_info.section_code,
                section_info.available_seats,
                section_info.total_seats,
                section_info.enrolled_ct,
                section_info.waitlist_ct,
                section_info  // Log the complete raw response
            );

            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open("section_details.log")?;

            writeln!(file, "{}", details)?;

            // Determine if we should attempt enrollment based on threshold
            // threshold = 0: Any availability (available_seats > 0)
            // threshold > 0: Seats available AND within threshold (0 < available_seats <= threshold)
            let has_availability = section_info.available_seats > 0;
            let within_threshold = seat_threshold == 0 || section_info.available_seats <= seat_threshold;
            let should_attempt = has_availability && within_threshold;

            if should_attempt {
                // Double-check the section immediately before returning
                let recheck = wrapper.req(term).parsed().get_course_info(department, course_code).await?;
                for recheck_info in recheck {
                    if recheck_info.section_code == section {
                        // Log the recheck
                        let recheck_details = format!(
                            "[{}] RECHECK {} {} Section {}:\n\
                            Available Seats: {} -> {}\n\
                            Enrolled Count: {} -> {}\n\
                            -------------------\n",
                            Local::now().format("%Y-%m-%d %H:%M:%S.%f"),
                            department,
                            course_code,
                            section,
                            section_info.available_seats,
                            recheck_info.available_seats,
                            section_info.enrolled_ct,
                            recheck_info.enrolled_ct,
                        );
                        writeln!(file, "{}", recheck_details)?;

                        // Recheck with same logic
                        let recheck_has_availability = recheck_info.available_seats > 0;
                        let recheck_within_threshold = seat_threshold == 0 || recheck_info.available_seats <= seat_threshold;
                        let recheck_should_attempt = recheck_has_availability && recheck_within_threshold;

                        // Only proceed if both checks show availability
                        if recheck_should_attempt {
                            let threshold_msg = if seat_threshold == 0 {
                                "Found opening!".to_string()
                            } else {
                                format!("Seats are at or below threshold ({})!", seat_threshold)
                            };
                            info!("🎯 {} Section {} has {} seats available (verified)",
                                threshold_msg, section, recheck_info.available_seats);
                            return Ok(Some(section_info.section_id.clone()));
                        } else {
                            info!("⚠️  False positive: Section {} showed availability but recheck failed",
                                section);
                            return Ok(None);
                        }
                    }
                }
            } else {
                println!("📍 {} {} Section {} - Full ({} enrolled/{} total) - Trying again in {} seconds",
                    department,
                    course_code,
                    section,
                    section_info.enrolled_ct,
                    section_info.total_seats,
                    polling_interval
                );
            }
        }
    }

    Ok(None)
}

pub async fn monitor_section_with_retry(
    wrapper: &WebRegWrapper,
    term: &str,
    section: &str,
    department: &str,
    course_code: &str,
    polling_interval: u64,
    seat_threshold: i64,
    notifier: &Notifier,
) -> Result<Option<String>, Box<dyn StdError + Send + Sync>> {
    let retry_strategy = get_retry_strategy();

    let result = tokio_retry::Retry::spawn(retry_strategy, || async {
        match monitor_section(wrapper, term, section, department, course_code, polling_interval, seat_threshold).await {
            Ok(result) => Ok(result),
            Err(e) => {
                warn!("Error monitoring section {}: {:?}, retrying...", section, e);
                Err(e)
            }
        }
    }).await?;

    if let Some(_section_id) = &result {
        let msg = format!(
            "Found opening in {} {} section {}!\n\nAttempting enrollment...\nTime: {}",
            department, course_code, section, Local::now().format("%Y-%m-%d %H:%M:%S")
        );
        notifier.send_notification(&msg).await;
    }

    Ok(result)
}
