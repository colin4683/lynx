use log::info;
use regex::Regex;
use reqwest::Client;
use sqlx::Row;
use std::fmt::Error;
use thiserror::Error;

#[derive(Debug)]
pub struct NotificationRule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub description: String,
    pub conditions: Vec<Condition>,
    pub actions: Vec<String>,
}

#[derive(Debug)]
pub struct Condition {
    pub component: String,
    pub metric: String,
    pub operator: String,
    pub value: String,
    pub next_compare: Option<String>,
}

pub async fn process_notification(
    metrics: &crate::proto::monitor::MetricsRequest,
    system_id: i32,
    pool: &sqlx::PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let alerts = sqlx::query(
        r#"
SELECT rule_id
FROM alert_systems
WHERE system_id = $1
"#,
    )
    .bind(system_id)
    .fetch_all(pool)
    .await?;

    let mut rules = Vec::new();
    for alert in alerts {
        let id_i32: i32 = alert.get("rule_id");

        let row = sqlx::query(
            r#"
SELECT id, name, active, expression, severity
FROM alert_rules
WHERE id=$1
AND active = true
"#,
        )
        .bind(id_i32)
        .fetch_one(pool)
        .await?;

        let id: i32 = row.get("id");
        let name: String = row.get("name");
        let enabled: bool = row.get("active");
        let expression: String = row.get("expression");
        let severity: String = row.get("severity");
        let mut conditions = Vec::new();
        let mut actions = Vec::new();

        // First split into component parts (e.g., "system.cpu > 50" -> ["system", "cpu > 50"])
        let component_re =
            Regex::new(r"^([a-zA-Z0-9_]+)\.([a-zA-Z0-9_]+)\s*([<>!=]+)\s*([a-zA-Z0-9_.]+)")
                .unwrap();

        // Split the expression at logical operators while preserving them
        let logical_re = Regex::new(r"\s+(AND|OR)\s+").unwrap();
        let segments: Vec<&str> = logical_re.split(&expression).collect();
        let operators: Vec<&str> = logical_re
            .find_iter(&expression)
            .map(|m| m.as_str().trim())
            .collect();

        for (i, segment) in segments.iter().enumerate() {
            if let Some(caps) = component_re.captures(segment) {
                let component = caps.get(1).unwrap().as_str().to_string();
                let metric = caps.get(2).unwrap().as_str().to_string();
                let operator = caps.get(3).unwrap().as_str().to_string();
                let value = caps.get(4).unwrap().as_str().to_string();
                let next_compare = if i < operators.len() {
                    Some(operators[i].to_string())
                } else {
                    None
                };
                conditions.push(Condition {
                    component,
                    metric,
                    operator,
                    value,
                    next_compare,
                });
            }
        }
        actions.push(severity);
        rules.push(NotificationRule {
            id: id.to_string(),
            name,
            enabled,
            description: "poop".to_string(),
            conditions,
            actions,
        });
    }

    info!("Rules: {:?}", rules);

    // Process the metrics and apply rules where applicable
    for rule in rules {
        let mut conditions_met = false;
        for condition in &rule.conditions {
            let metric_value = match condition.component.as_str() {
                "cpu" => metrics.cpu_stats.unwrap().usage_percent,
                "memory" => {
                    metrics.memory_stats.unwrap().used_kb as f64
                        / metrics.memory_stats.unwrap().total_kb as f64
                        * 100.0
                }
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

            info!(
                "Evaluating condition: {} {} {} -> Result: {}",
                metric_value, condition.operator, condition.value, comparison_result
            );

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
                        if let Err(e) = NotificationService::Email(email_config)
                            .send("Test email message")
                            .await
                        {
                            info!("Failed to send email: {}", e);
                        } else {
                            info!("Email sent successfully");
                        }
                    }
                    "low" => {
                        let discord_config = DiscordConfig {
                            webhook_url: "https://discord.com/api/webhooks/1397708278192148560/Z81BCG2mju3DNlD-uraLjrnk5wPGwKYvjXCIKXmy8wwy2qbvOtSGFrz9KtkAW85FxSzU".to_string(),
                        };
                        let message = format!(
                            r##"
**Low Severity Alert**: A condition has been met that requires attention.
Rule Name: {}
Description: {}
Severity: Low
                        "##,
                            rule.name, rule.description
                        );
                        if let Err(e) = NotificationService::Discord(discord_config)
                            .send(&*message)
                            .await
                        {
                            info!("Failed to send Discord message: {}", e);
                        } else {
                            info!("Discord message sent successfully");
                        }
                    }
                    "slack" => {
                        let slack_config = SlackConfig {
                            webhook_url: "https://hooks.slack.com/services/your_webhook_url"
                                .to_string(),
                        };
                        if let Err(e) = NotificationService::Slack(slack_config)
                            .send("Test Slack message")
                            .await
                        {
                            info!("Failed to send Slack message: {}", e);
                        } else {
                            info!("Slack message sent successfully");
                        }
                    }
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
    pub webhook_url: String,
}

pub struct SlackConfig {
    pub webhook_url: String,
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
        }
        "discord" => {
            let mut params = details.split('@');
            let token = params.next()?;
            let webhook_id = params.next()?;

            Some(NotificationService::Discord(DiscordConfig {
                webhook_url: format!("https://discord.com/api/webhooks/{}/{}", webhook_id, token),
            }))
        }
        _ => None,
    }
}

fn send_email(config: &EmailConfig, body: &str) -> Result<(), NotificationError> {
    // Implement email sending logic here
    Ok(())
}

async fn send_discord(config: &DiscordConfig, message: &str) -> Result<(), NotificationError> {
    // Implement Discord message sending logic here
    let client = Client::new();
    let res = client
        .post(&config.webhook_url)
        .json(&serde_json::json!({ "content": message }))
        .send()
        .await?;
    Ok(())
}

async fn send_slack(config: &SlackConfig, message: &str) -> Result<(), NotificationError> {
    // Implement Slack message sending logic here
    Ok(())
}
