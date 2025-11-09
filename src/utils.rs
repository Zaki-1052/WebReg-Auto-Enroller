use std::time::Duration;
use std::error::Error as StdError;
use std::fs::OpenOptions;
use log::LevelFilter;
use env_logger::Builder;
use tokio_retry::strategy::{ExponentialBackoff, jitter};
use crate::config::{DEFAULT_RETRY_ATTEMPTS, DEFAULT_RETRY_DELAY};

pub fn setup_logging() -> Result<(), Box<dyn StdError + Send + Sync>> {
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

pub fn get_retry_strategy() -> impl Iterator<Item = Duration> {
    ExponentialBackoff::from_millis(DEFAULT_RETRY_DELAY)
        .factor(2)
        .max_delay(Duration::from_secs(60))
        .map(jitter)
        .take(DEFAULT_RETRY_ATTEMPTS as usize)
}

pub fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    format!("{}h {}m {}s", hours, minutes, seconds)
}
