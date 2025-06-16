use std::fmt::Error;
use log::info;
use thiserror::Error;
use reqwest::Client;

#[derive(Debug)]
pub struct NotificationRule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub description: String,
    pub conditions: Vec<Condition>,
    pub actions: Vec<String>
}

#[derive(Debug)]
pub struct Condition {
    pub component: String,
    pub metric: String,
    pub operator: String,
    pub value: String,
    pub next_compare: Option<String>
}

pub async fn process_notification(
    metrics: &crate::proto::monitor::MetricsRequest
) -> Result<(), Box<dyn std::error::Error>> {

    let rules = vec![
        NotificationRule {
            id: "1".to_string(),
            name: "Test Rule 1".to_string(),
            enabled: true,
            description: "Test Rule".to_string(),
            conditions: vec![
                Condition {
                    component: "cpu".to_string(),
                    metric: "usage".to_string(),
                    operator: ">".to_string(),
                    value: "60".to_string(),
                    next_compare: Some("and".to_string())
                },
                Condition {
                    component: "memory".to_string(),
                    metric: "usage".to_string(),
                    operator: "<".to_string(),
                    value: "70".to_string(),
                    next_compare: None
                }
            ],
            actions: vec!["discord".to_string()]
        }
    ];

    // Process the metrics and apply rules where applicable
    for rule in rules {
        let mut conditions_met = false;
        for condition in &rule.conditions {
            let metric_value = match condition.component.as_str() {
                "cpu" => metrics.cpu_stats.unwrap().usage_percent,
                "memory" => metrics.memory_stats.unwrap().used_kb as f64 / metrics.memory_stats.unwrap().total_kb as f64 * 100.0,
                _ => return Err("Unknown component".into()),
            };
            let comparison_result = match condition.operator.as_str() {
                ">" => metric_value > condition.value.parse::<f64>()?,
                "<" => metric_value < condition.value.parse::<f64>()?,
                ">=" => metric_value >= condition.value.parse::<f64>()?,
                "<=" => metric_value <= condition.value.parse::<f64>()?,
                "==" => metric_value == condition.value.parse::<f64>()?,
                "!=" => metric_value != condition.value.parse::<f64>()?,
                _ => return Err("Invalid operator".into()),
            };

            info!("Evaluating condition: {} {} {} -> Result: {}",
                metric_value, condition.operator, condition.value, comparison_result);

            conditions_met = comparison_result;

            // Handle next_compare logic if applicable
            if let Some(next_compare) = &condition.next_compare {
                if next_compare == "and" && !conditions_met {
                    break; // If 'and' is specified and conditions are not met, exit
                } else if next_compare == "or" && conditions_met {
                    continue; // If 'or' is specified and conditions are met, continue
                }
            }
        }
        if conditions_met {
            for action in &rule.actions {
                match action.as_str() {
                    "email" => {
                        let email_config = EmailConfig {
                            smtp_server: "smtp.example.com".to_string(),
                            smtp_port: 587,
                            username: "test".to_string(),
                            password: "password".to_string(),
                            from_email: "from@email.com".to_string(),
                            to_email: "to@email.com".to_string(),
                            subject: "Notification Alert".to_string(),
                        };
                        if let Err(e) = NotificationService::Email(email_config).send("Test email message").await {
                            info!("Failed to send email: {}", e);
                        } else {
                            info!("Email sent successfully");
                        }
                    },
                    "discord" => {
                        let discord_config = DiscordConfig {
                            webhook_url: "https://discord.com/api/webhooks/1366492231350882324/mXVQevFjJERDaIfZ-GLeIbQY1vXDKZZMkmdoT19vBtmR8mxIxW7UBGgp-eJtj97aTfk8".to_string(),
                        };
                        if let Err(e) = NotificationService::Discord(discord_config).send("Test Discord message").await {
                            info!("Failed to send Discord message: {}", e);
                        } else {
                            info!("Discord message sent successfully");
                        }
                    },
                    "slack" => {
                        let slack_config = SlackConfig {
                            webhook_url: "https://hooks.slack.com/services/your_webhook_url".to_string(),
                        };
                        if let Err(e) = NotificationService::Slack(slack_config).send("Test Slack message").await {
                            info!("Failed to send Slack message: {}", e);
                        } else {
                            info!("Slack message sent successfully");
                        }
                    },
                    _ => info!("Unknown action: {}", action),
                }
            }
        }
    }

    Ok(())

}
#[derive(Error, Debug)]
pub enum NotificationError {
    #[error("Email sending error: {0}")]
    EmailError(#[from] lettre::transport::smtp::Error),

    #[error("HTTP request error: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

pub enum NotificationService {
    Email(EmailConfig),
    Discord(DiscordConfig),
    Slack(SlackConfig),
}


pub struct EmailConfig {
    pub smtp_server: String,
    pub smtp_port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
    pub to_email: String,
    pub subject: String,
}

pub struct DiscordConfig {
    pub webhook_url: String
}

pub struct SlackConfig {
    pub webhook_url: String
}

impl NotificationService {
    pub async fn send(&self, message: &str) -> Result<(), NotificationError> {
        match self {
            NotificationService::Email(config) => send_email(config, message),
            NotificationService::Discord(config) => send_discord(config, message).await,
            NotificationService::Slack(config) => send_slack(config, message).await,
        }
    }
}

fn create_sender(args: &str) -> Option<NotificationService> {
    /*
    Parses string argument to create a NotificationService instance.
    Examples:
        Email: "smtp://username:password@host:port/?from=from_email&to=to_email&subject=subject"
        Discord: "discord://token@webhook_id"
        Slack: "slack://hook:token@webhook
     */

    let parts: Vec<&str> = args.split("://").collect();
    if parts.len() != 2 {
        return None;
    }
    let service_type = parts[0];
    let details = parts[1];

    match service_type {
        "smtp" => {
            let mut params = details.split('@');
            let credentials = params.next()?;
            let server_info = params.next()?;

            let mut cred_parts = credentials.split(':');
            let username = cred_parts.next()?.to_string();
            let password = cred_parts.next()?.to_string();

            let mut server_parts = server_info.split('/');
            let server_address = server_parts.next()?.to_string();
            let from_email = server_parts.next()?.to_string();
            let to_email = server_parts.next()?.to_string();
            let subject = server_parts.next()?.to_string();

            Some(NotificationService::Email(EmailConfig {
                smtp_server: server_address,
                smtp_port: 587, // Default SMTP port
                username,
                password,
                from_email,
                to_email,
                subject,
            }))
        },
        "discord" => {
            let mut params = details.split('@');
            let token = params.next()?;
            let webhook_id = params.next()?;

            Some(NotificationService::Discord(DiscordConfig {
                webhook_url: format!("https://discord.com/api/webhooks/{}/{}", webhook_id, token),
            }))
        },
        _ => None
    }
}

fn send_email(config: &EmailConfig, body: &str) -> Result<(), NotificationError> {
    // Implement email sending logic here
    Ok(())
}

async fn send_discord(config: &DiscordConfig, message: &str) -> Result<(), NotificationError> {
    // Implement Discord message sending logic here
    let client = Client::new();
    let res = client.post(&config.webhook_url)
        .json(&serde_json::json!({ "content": message }))
        .send()
        .await?;
    Ok(())
}

async fn send_slack(config: &SlackConfig, message: &str) -> Result<(), NotificationError> {
    // Implement Slack message sending logic here
    Ok(())
}