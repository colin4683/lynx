use super::*;
use async_trait::async_trait;
use log::info;
use mail_send::{mail_builder::MessageBuilder, Credentials, SmtpClientBuilder};
use reqwest::Client;
use serde_json::json;
use url::Url;

#[derive(Error, Debug)]
pub enum NotificationError {
    #[error("Email sending error: {0}")]
    EmailError(#[from] mail_send::Error),
    #[error("HTTP request error: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("URL parsing error: {0}")]
    UrlError(#[from] url::ParseError),
}

// Enum to handle different notification service types
#[derive(Clone)]
pub enum NotificationServiceType {
    Discord(DiscordService),
    Email(EmailService),
}

#[async_trait]
impl NotificationService for NotificationServiceType {
    async fn send(&self, message: &str) -> Result<(), NotificationError> {
        match self {
            NotificationServiceType::Discord(discord) => discord.send(message).await,
            NotificationServiceType::Email(email) => email.send(message).await,
        }
    }
}

impl NotificationServiceType {
    pub fn from_url(url: &str) -> Result<Self, NotificationError> {
        if url.starts_with("discord://") {
            Ok(NotificationServiceType::Discord(DiscordService::from_url(
                url,
            )?))
        } else if url.starts_with("smtp://") {
            Ok(NotificationServiceType::Email(EmailService::from_url(url)?))
        } else {
            Err(NotificationError::ConfigError(format!(
                "Unsupported notification service: {}",
                url
            )))
        }
    }
}

// Discord notification service
#[derive(Clone)]
pub struct DiscordService {
    webhook_url: String,
    username: String,
}

impl DiscordService {
    pub fn new(webhook_url: String, username: String) -> Self {
        Self {
            webhook_url,
            username,
        }
    }

    pub fn from_url(url: &str) -> Result<Self, NotificationError> {
        let url = urlencoding::decode(url)
            .map_err(|_| {
                NotificationError::ConfigError("Failed to decode Discord webhook URL".to_string())
            })?
            .to_string();
        let parts: Vec<&str> = url.split("://").collect();

        info!("url: {}", url);
        info!("Parsed Discord webhook URL parts: {:?}", parts);

        // Extract channel_id and token from URL path
        let (channel_id, token) = match (parts.get(parts.len() - 2), parts.get(parts.len() - 1)) {
            (Some(channel), Some(tok)) => (channel, tok),
            _ => {
                return Err(NotificationError::ConfigError(
                    "Invalid Discord webhook URL".to_string(),
                ));
            }
        };

        let username = url
            .split('?')
            .nth(1)
            .and_then(|q| q.split('=').nth(1).map(|u| u.replace('+', " ")))
            .unwrap_or_else(|| "Lynx Monitor".to_string());

        let webhook_url = format!("https://discord.com/api/webhooks/{}/{}", channel_id, token);
        Ok(Self::new(webhook_url, username))
    }
}

#[async_trait]
impl NotificationService for DiscordService {
    async fn send(&self, message: &str) -> Result<(), NotificationError> {
        let client = Client::new();
        let payload = json!({
            "username": self.username,
            "embeds": [{
                "title": "Lynx Monitor Alert",
                "description": message,
                "color": 16711680
            }]
        });

        info!("Sending Discord notification to {}", self.webhook_url);
        client.post(&self.webhook_url).json(&payload).send().await?;

        Ok(())
    }
}

// Email notification service
#[derive(Clone)]
pub struct EmailService {
    smtp_server: String,
    smtp_port: u16,
    username: String,
    password: String,
    from_email: String,
    to_email: String,
    subject: String,
}

impl EmailService {
    pub fn new(
        smtp_server: String,
        smtp_port: u16,
        username: String,
        password: String,
        from_email: String,
        to_email: String,
        subject: String,
    ) -> Self {
        Self {
            smtp_server,
            smtp_port,
            username,
            password,
            from_email,
            to_email,
            subject,
        }
    }

    pub fn from_url(url: &str) -> Result<Self, NotificationError> {
        let url = urlencoding::decode(url)
            .map_err(|_| NotificationError::ConfigError("Failed to decode email URL".to_string()))?
            .to_string();
        let url = Url::parse(url.as_str())?;

        if url.scheme() != "smtp" {
            return Err(NotificationError::ConfigError(
                "Invalid email URL scheme".to_string(),
            ));
        }

        let username = url.username();
        let password = url
            .password()
            .ok_or_else(|| NotificationError::ConfigError("Missing SMTP password".to_string()))?;
        let smtp_server = url
            .host_str()
            .ok_or_else(|| NotificationError::ConfigError("Missing SMTP server".to_string()))?
            .to_string();
        let smtp_port = url
            .port()
            .ok_or_else(|| NotificationError::ConfigError("Missing SMTP port".to_string()))?;

        let params: std::collections::HashMap<String, String> = url
            .query_pairs()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        let from_email = params
            .get("from")
            .ok_or_else(|| NotificationError::ConfigError("Missing from email".to_string()))?
            .clone();
        let to_email = params
            .get("to")
            .ok_or_else(|| NotificationError::ConfigError("Missing to email".to_string()))?
            .clone();
        let subject = params
            .get("subject")
            .map(|s| s.clone())
            .unwrap_or_else(|| "Lynx Monitor Alert".to_string());

        info!(
            "Sending email with info: smtp_server={}, smtp_port={}, username={}, from_email={}, to_email={}, subject={}",
            smtp_server, smtp_port, username, from_email, to_email, subject
        );

        Ok(Self::new(
            smtp_server,
            smtp_port,
            username.to_string(),
            password.to_string(),
            from_email,
            to_email,
            subject,
        ))
    }
}

#[async_trait]
impl NotificationService for EmailService {
    async fn send(&self, message: &str) -> Result<(), NotificationError> {
        let message = MessageBuilder::new()
            .from(self.from_email.clone())
            .to(self.to_email.clone())
            .subject(self.subject.clone())
            .text_body(message.to_string());

        let credentials = Credentials::Plain {
            username: &self.username,
            secret: &self.password,
        };

        SmtpClientBuilder::new(&self.smtp_server, self.smtp_port)
            .implicit_tls(false)
            .credentials(credentials)
            .connect()
            .await?
            .send(message)
            .await?;

        Ok(())
    }
}
