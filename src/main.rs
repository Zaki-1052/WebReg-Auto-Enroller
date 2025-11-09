use std::time::{Duration, SystemTime};
use tokio::time::sleep;
use webweg::wrapper::{WebRegWrapper, input_types::{AddType, EnrollWaitAdd, GradeOption}};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use tokio::signal::ctrl_c;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::fs;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use log::{LevelFilter, info, warn, error};
use env_logger::Builder;
use tokio_retry::strategy::{ExponentialBackoff, jitter};
use std::fs::OpenOptions;
use reqwest::Client as HttpClient;
use std::path::Path;
use std::error::Error as StdError;

// Constants
const DEFAULT_RETRY_ATTEMPTS: u32 = 3;
const DEFAULT_RETRY_DELAY: u64 = 1000;
const CONFIG_PATH: &str = "config.toml";

#[derive(Debug, Deserialize, Clone)]
struct AppConfig {
    webreg: WebRegConfig,
    notifications: NotificationConfig,
    courses: CourseConfig,
    monitoring: MonitoringConfig,
}

#[derive(Debug, Deserialize, Clone)]
struct WebRegConfig {
    term: String,
    polling_interval: u64,
    cookie: String,
}

#[derive(Debug, Deserialize, Clone)]
struct NotificationConfig {
    gmail_address: String,
    gmail_app_password: String,
    email_recipients: Vec<String>,
    discord_webhook_url: String,
}

#[derive(Debug, Deserialize, Clone)]
struct CourseConfig {
    chem: CourseDetails,
    bild: LegacyCourseDetails,  // Use old format for BILD
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]  // This allows serde to try both formats
enum CourseDetails {
    New(NewCourseDetails),
    Legacy(LegacyCourseDetails),
}

#[derive(Debug, Deserialize, Clone)]
struct NewCourseDetails {
    department: String,
    course_code: String,
    sections: Vec<SectionGroup>,
}

#[derive(Debug, Deserialize, Clone)]
struct LegacyCourseDetails {
    department: String,
    course_code: String,
    lecture_section: String,
    discussion_sections: Vec<String>,
}

impl CourseDetails {
    fn department(&self) -> &str {
        match self {
            CourseDetails::New(details) => &details.department,
            CourseDetails::Legacy(details) => &details.department,
        }
    }

    fn course_code(&self) -> &str {
        match self {
            CourseDetails::New(details) => &details.course_code,
            CourseDetails::Legacy(details) => &details.course_code,
        }
    }
}

fn to_section_groups(course: &LegacyCourseDetails) -> Vec<SectionGroup> {
    vec![SectionGroup {
        lecture: course.lecture_section.clone(),
        discussions: course.discussion_sections.clone(),
    }]
}

#[derive(Debug, Deserialize, Clone)]
struct SectionGroup {
    lecture: String,
    discussions: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct MonitoringConfig {
    log_file: String,
    stats_file: String,
    cookie_refresh_interval: u64,
    max_retries: u32,
    retry_delay: u64,
    #[serde(default = "default_seat_threshold")]
    seat_threshold: i64,  // Threshold for available seats (0 = any availability, 3 = fewer than 3 seats)
}

fn default_seat_threshold() -> i64 {
    0  // Default to aggressive mode (any seat availability)
}

use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SectionFailures {
    count: u64,
    last_failure: DateTime<Local>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct EnrollmentStats {
    total_checks: u64,
    openings_found: u64,
    enrollment_attempts: u64,
    successful_enrollments: u64,
    errors: u64,
    last_updated: String,
    start_time: String,
    section_failures: HashMap<String, SectionFailures>,  // Track failures per section
}

impl EnrollmentStats {
    fn should_notify_for_section(&mut self, section_id: &str) -> bool {
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
struct HealthStatus {
    uptime: String,
    last_successful_check: String,
    connection_status: bool,
    error_count: u64,
    success_rate: f64,
    total_checks: u64,
}

struct Notifier {
    smtp_transport: SmtpTransport,
    http_client: HttpClient,
    config: NotificationConfig,
}

impl Clone for Notifier {
    fn clone(&self) -> Self {
        Self {
            smtp_transport: self.smtp_transport.clone(),
            http_client: self.http_client.clone(),
            config: self.config.clone(),
        }
    }
}

impl Notifier {
    fn new(config: &NotificationConfig) -> Result<Self, Box<dyn StdError + Send + Sync>> {
        let creds = Credentials::new(
            config.gmail_address.clone(),
            config.gmail_app_password.clone(),
        );

        let smtp_transport = SmtpTransport::relay("smtp.gmail.com")
            .unwrap()
            .credentials(creds)
            .build();

        let http_client = HttpClient::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        Ok(Self {
            smtp_transport,
            http_client,
            config: config.clone(),
        })
    }

    async fn send_notification(&self, message: &str) {
        self.send_email(message).await;
        self.send_discord(message).await;
        info!("Notification sent: {}", message);
    }

    async fn send_email(&self, content: &str) {
        for recipient in &self.config.email_recipients {
            let email = Message::builder()
                .from(format!("WebReg Monitor <{}>", self.config.gmail_address).parse().unwrap())
                .to(recipient.parse().unwrap())
                .subject("WebReg Course Opening Alert!")
                .body(content.to_string())
                .unwrap();

            match self.smtp_transport.send(&email) {
                Ok(_) => info!("📧 Email sent to {}", recipient),
                Err(e) => error!("Could not send email to {}: {:?}", recipient, e),
            }
        }
    }

    async fn send_discord(&self, content: &str) {
        let payload = serde_json::json!({
            "content": content,
            "username": "WebReg Monitor",
            "avatar_url": "https://ucsd.edu/favicon.ico"
        });

        match self.http_client.post(&self.config.discord_webhook_url)
            .json(&payload)
            .send()
            .await {
                Ok(_) => info!("Discord webhook message sent"),
                Err(e) => error!("Could not send Discord webhook: {:?}", e),
            }
    }
}

struct AppState {
    stats: EnrollmentStats,
    config: AppConfig,
    notifier: Notifier,
    wrapper: WebRegWrapper,
    start_time: SystemTime,
    last_check_time: String,
    is_connected: bool,
    term: String,
}

impl AppState {
    async fn new() -> Result<Self, Box<dyn StdError + Send + Sync>> {
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
        section_failures: HashMap::new(),  // Add this line
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

fn clone_wrapper(&self) -> WebRegWrapper {
        WebRegWrapper::builder()
            .with_cookies(&self.config.webreg.cookie)
            .try_build_wrapper()
            .expect("Failed to clone WebRegWrapper")
    }

    fn update_stats(&mut self) {
        self.stats.last_updated = Local::now().to_string();
        let stats_json = serde_json::to_string_pretty(&self.stats).unwrap();
        if let Err(e) = fs::write(&self.config.monitoring.stats_file, stats_json) {
            error!("Failed to write stats file: {:?}", e);
        }
    }

    async fn check_health(&self) -> HealthStatus {
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

    async fn monitor_section_health(
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

// Core WebReg Functions
async fn initialize_webreg(config: &WebRegConfig) -> Result<WebRegWrapper, Box<dyn StdError + Send + Sync>> {
    println!("Starting initialize_webreg");
    println!("Cookie length: {}", config.cookie.len());
    //println!("Term: {}", config.term);

    let wrapper = WebRegWrapper::builder()
        .with_cookies(&config.cookie)
        .try_build_wrapper()
        .ok_or("Failed to build WebReg wrapper")?;

    println!("Successfully built wrapper, attempting to associate term");

    let result = wrapper.associate_term(&config.term).await;
    match &result {
        Ok(_) => println!("Successfully associated term"),
        Err(e) => println!("Error associating term: {:?}", e),
    }

    result?;
    info!("Successfully initialized WebReg connection for term {}", config.term);

    Ok(wrapper)
}

fn setup_logging() -> Result<(), Box<dyn StdError + Send + Sync>> {
    let mut builder = Builder::from_default_env();
    builder.filter_level(LevelFilter::Info);

    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("webreg_monitor.log")?;

    builder.target(env_logger::Target::Pipe(Box::new(log_file)));
    builder.init();

    Ok(())
}

fn get_retry_strategy() -> impl Iterator<Item = Duration> {
    ExponentialBackoff::from_millis(DEFAULT_RETRY_DELAY)
        .factor(2)
        .max_delay(Duration::from_secs(60))
        .map(jitter)
        .take(DEFAULT_RETRY_ATTEMPTS as usize)
}

async fn monitor_section(
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

            use std::io::Write;
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

fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    format!("{}h {}m {}s", hours, minutes, seconds)
}

async fn monitor_section_with_retry(
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

async fn try_enroll_with_retry(
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

async fn try_enroll(
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

async fn is_connection_valid(wrapper: &WebRegWrapper, term: &str) -> bool {
    match wrapper.associate_term(term).await {
        Ok(_) => true,
        Err(_) => false
    }
}

async fn refresh_cookie(state: &mut AppState) -> Result<(), Box<dyn StdError + Send + Sync>> {
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

async fn run_monitor(
    state: Arc<Mutex<AppState>>,
    shutdown: tokio::sync::broadcast::Receiver<()>,
) -> Result<(), Box<dyn StdError + Send + Sync>> {
    let mut shutdown_rx = shutdown;
    let mut cookie_refresh_timer = tokio::time::interval(
        Duration::from_secs(state.lock().await.config.monitoring.cookie_refresh_interval)
    );

    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                info!("Received shutdown signal, stopping monitoring...");
                break;
            }
            _ = cookie_refresh_timer.tick() => {
                let mut state_guard = state.lock().await;
                if let Err(e) = refresh_cookie(&mut state_guard).await {
                    error!("Failed to refresh cookie: {:?}", e);
                    continue;
                }
            }
            _ = async {
                let mut state_guard = state.lock().await;

                // Skip monitoring if not connected
                if !state_guard.is_connected {
                    sleep(Duration::from_secs(state_guard.config.webreg.polling_interval)).await;
                    return;
                }

                // Clone all the values we need
                let term = state_guard.term.clone();
                let wrapper = state_guard.clone_wrapper();
                let notifier = state_guard.notifier.clone();
                let chem_config = state_guard.config.courses.chem.clone();
                let bild_config = state_guard.config.courses.bild.clone();
                let polling_interval = state_guard.config.webreg.polling_interval;
                let seat_threshold = state_guard.config.monitoring.seat_threshold;

                // Monitor CHEM sections
                let chem_sections = match &chem_config {
                    CourseDetails::New(details) => details.sections.clone(),
                    CourseDetails::Legacy(details) => to_section_groups(details),
                };

                for section_group in &chem_sections {
                // Monitor lecture section
                if let Ok(Some(section_id)) = monitor_section_with_retry(
                    &wrapper,
                    &term,
                    &section_group.lecture,
                    &chem_config.department(),
                    &chem_config.course_code(),
                    polling_interval,
                    seat_threshold,
                    &notifier,
                ).await {
                    state_guard.stats.enrollment_attempts += 1;
                    if let Ok(true) = try_enroll_with_retry(
                        &wrapper,
                        &term,
                        &section_id,
                        &chem_config.department(),
                        &chem_config.course_code(),
                        &section_group.lecture,
                        &notifier,
                        &mut state_guard.stats,
                    ).await {
                        state_guard.stats.successful_enrollments += 1;
                    }
                }

                // Monitor discussion sections
                for discussion in &section_group.discussions {
                    if let Ok(Some(section_id)) = monitor_section_with_retry(
                        &wrapper,
                        &term,
                        discussion,
                        &chem_config.department(),
                        &chem_config.course_code(),
                        polling_interval,
                        seat_threshold,
                        &notifier,
                    ).await {
                        state_guard.stats.enrollment_attempts += 1;
                        if let Ok(true) = try_enroll_with_retry(
                            &wrapper,
                            &term,
                            &section_id,
                            &chem_config.department(),
                            &chem_config.course_code(),
                            discussion,
                            &notifier,
                            &mut state_guard.stats,
                        ).await {
                            state_guard.stats.successful_enrollments += 1;
                        }
                    }
                }
            }

            // Monitor BILD sections
            let bild_sections = to_section_groups(&bild_config);

            for section_group in &bild_sections {
                // Monitor lecture section
                if let Ok(Some(section_id)) = monitor_section_with_retry(
                    &wrapper,
                    &term,
                    &section_group.lecture,
                    &bild_config.department,
                    &bild_config.course_code,
                    polling_interval,
                    seat_threshold,
                    &notifier,
                ).await {
                    state_guard.stats.enrollment_attempts += 1;
                    if let Ok(true) = try_enroll_with_retry(
                        &wrapper,
                        &term,
                        &section_id,
                        &bild_config.department,
                        &bild_config.course_code,
                        &section_group.lecture,
                        &notifier,
                        &mut state_guard.stats,
                    ).await {
                        state_guard.stats.successful_enrollments += 1;
                    }
                }

                // Monitor discussion sections
                for discussion in &section_group.discussions {
                    if let Ok(Some(section_id)) = monitor_section_with_retry(
                        &wrapper,
                        &term,
                        discussion,
                        &bild_config.department,
                        &bild_config.course_code,
                        polling_interval,
                        seat_threshold,
                        &notifier,
                    ).await {
                        state_guard.stats.enrollment_attempts += 1;
                        if let Ok(true) = try_enroll_with_retry(
                            &wrapper,
                            &term,
                            &section_id,
                            &bild_config.department,
                            &bild_config.course_code,
                            discussion,
                            &notifier,
                            &mut state_guard.stats,
                        ).await {
                            state_guard.stats.successful_enrollments += 1;
                        }
                    }
                }
            }

                let health = state_guard.check_health().await;
                info!("Health status: {:?}", health);
                state_guard.last_check_time = Local::now().to_string();

                sleep(Duration::from_secs(polling_interval)).await;
            } => {}
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn StdError + Send + Sync>> {
    println!("Starting main...");

    // Setup logging
    println!("Setting up logging...");
    setup_logging()?;
    info!("Starting WebReg monitoring system...");

    // Initialize application state
    println!("Initializing application state...");
    let state = Arc::new(Mutex::new(AppState::new().await?));
    println!("Application state initialized successfully");

    // Setup shutdown channel
    println!("Setting up shutdown channel...");
    let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);

    // Print startup information
    {
        println!("Printing startup information...");
        let state_guard = state.lock().await;
        info!("Monitoring the following sections:");
        
        match &state_guard.config.courses.chem {
            CourseDetails::New(details) => {
                for section_group in &details.sections {
                    info!("CHEM {}: Lecture {} with discussions {:?}", 
                        details.course_code,
                        section_group.lecture,
                        section_group.discussions);
                }
            },
            CourseDetails::Legacy(details) => {
                info!("CHEM {}: Lecture {} with discussions {:?}",
                    details.course_code,
                    details.lecture_section,
                    details.discussion_sections);
            }
        }
        
        info!("BILD {}: Lecture {} with discussions {:?}",
            state_guard.config.courses.bild.course_code,
            state_guard.config.courses.bild.lecture_section,
            state_guard.config.courses.bild.discussion_sections);
        
        info!("Checking every {} seconds", state_guard.config.webreg.polling_interval);
        info!("Press Ctrl+C to stop the program");
    }

    // Spawn monitoring task
    println!("Spawning monitoring task...");
    let monitor_handle = tokio::spawn(run_monitor(state.clone(), shutdown_rx));
    println!("Monitor task spawned, waiting for Ctrl+C...");

    // Wait for Ctrl+C
    tokio::select! {
        _ = ctrl_c() => {
            println!("Received Ctrl+C signal");
            info!("Received Ctrl+C, initiating graceful shutdown...");
            let _ = shutdown_tx.send(());
        }
    }

    // Wait for monitor to finish
    println!("Waiting for monitor to finish...");
    monitor_handle.await??;

    // Final stats update
    {
        println!("Updating final stats...");
        let mut state_guard = state.lock().await;
        state_guard.update_stats();
        let health = state_guard.check_health().await;
        info!("Final health status: {:?}", health);
    }

    println!("Shutdown complete!");
    info!("Shutdown complete!");
    Ok(())
}
