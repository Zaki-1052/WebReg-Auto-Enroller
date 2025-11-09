use std::time::Duration;
use std::error::Error as StdError;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use reqwest::Client as HttpClient;
use log::{info, error};
use crate::config::NotificationConfig;

pub struct Notifier {
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
    pub fn new(config: &NotificationConfig) -> Result<Self, Box<dyn StdError + Send + Sync>> {
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

    pub async fn send_notification(&self, message: &str) {
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
