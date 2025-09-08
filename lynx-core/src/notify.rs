use log::info;
use regex::Regex;
use reqwest::Client;
use sqlx::Row;
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
            NotificationService::Email(_config) => Ok(()),
            NotificationService::Discord(config) => send_discord(config, message).await,
            NotificationService::Slack(_config) => Ok(()),
        }
    }
}

async fn send_discord(config: &DiscordConfig, message: &str) -> Result<(), NotificationError> {
    let client = Client::new();
    let second_half = config.webhook_url.split("://").nth(1).ok_or_else(|| {
        NotificationError::ConfigError("Invalid Discord webhook URL format".to_string())
    })?;
    let token = second_half.split('@').nth(0).ok_or_else(|| {
        NotificationError::ConfigError("Invalid Discord webhook URL format".to_string())
    })?;
    let third_half = second_half.split('@').nth(1).ok_or_else(|| {
        NotificationError::ConfigError("Invalid Discord webhook URL format".to_string())
    })?;
    let channel_id = third_half.split('?').nth(0).ok_or_else(|| {
        NotificationError::ConfigError("Invalid Discord webhook URL format".to_string())
    })?;
    let username = third_half
        .split('?')
        .nth(1)
        .and_then(|q| q.split('=').nth(1).map(|u| u.replace('+', " ")))
        .unwrap_or_else(|| "Lynx Monitor".to_string());
    let webhook_url = format!("https://discord.com/api/webhooks/{}/{}", channel_id, token);

    // build embedded message
    let payload = serde_json::json!({
        "username": username,
        "embeds": [{
            "title": "Lynx Monitor Alert",
            "description": message,
            "color": 16711680
        }]
    });
    let res = client.post(&webhook_url).json(&payload).send().await?;

    Ok(())
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

        let notifiers = sqlx::query(
            r#"
SELECT rule_id, notifier_id
FROM alert_notifiers
WHERE
rule_id = $1
"#,
        )
        .bind(id_i32)
        .fetch_all(pool)
        .await?;

        let id: i32 = row.get("id");
        let name: String = row.get("name");
        let enabled: bool = row.get("active");
        let expression: String = row.get("expression");
        let severity: String = row.get("severity");
        let mut conditions = Vec::new();
        let mut actions = Vec::new();

        let component_re =
            Regex::new(r"^([a-zA-Z0-9_]+)\.([a-zA-Z0-9_]+)\s*([<>!=]+)\s*([a-zA-Z0-9_.]+)")
                .unwrap();
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

            for notifier in &notifiers {
                let notifier_id: i32 = notifier.get("notifier_id");
                let notifier_row = sqlx::query(
                    r#"
SELECT id, type, value
FROM notifiers
WHERE id = $1
"#,
                )
                .bind(notifier_id)
                .fetch_one(pool)
                .await?;

                let notifier_type: String = notifier_row.get("type");
                let notifier_value: String = notifier_row.get("value");
                actions.push(format!("{}:{}", notifier_type, notifier_value));
            }
        }

        rules.push(NotificationRule {
            id: id.to_string(),
            name,
            enabled,
            description: "poop".to_string(),
            conditions,
            actions,
        });
    }

    for rule in rules {
        let mut conditions_met = false;
        for condition in &rule.conditions {
            let metric_value = match condition.component.as_str() {
                "cpu" => match condition.metric.as_str() {
                    "usage" => metrics.cpu_stats.unwrap().usage_percent as f64,
                    _ => return Err("Unknown CPU metric".into()),
                },
                "memory" => match condition.metric.as_str() {
                    "used" => metrics.memory_stats.unwrap().used_kb as f64,
                    "total" => metrics.memory_stats.unwrap().total_kb as f64,
                    "usage" => {
                        metrics.memory_stats.unwrap().used_kb as f64
                            / metrics.memory_stats.unwrap().total_kb as f64
                            * 100.0
                    }
                    _ => return Err("Unknown Memory metric".into()),
                },
                "load" => match condition.metric.as_str() {
                    "one" => metrics.load_average.unwrap().one_minute as f64,
                    "five" => metrics.load_average.unwrap().five_minutes as f64,
                    "fifteen" => metrics.load_average.unwrap().fifteen_minutes as f64,
                    _ => return Err("Unknown Load metric".into()),
                },
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
            conditions_met = comparison_result;
            if let Some(next_compare) = &condition.next_compare {
                if next_compare == "and" && !conditions_met {
                    break;
                } else if next_compare == "or" && conditions_met {
                    continue;
                }
            }
        }
        if conditions_met {
            for action in &rule.actions {
                let parts: Vec<&str> = action.splitn(2, ':').collect();
                if parts.len() != 2 {
                    continue;
                }
                let notifier_type = parts[0];
                let notifier_value = parts[1];

                let service = match notifier_type {
                    "Discord" => NotificationService::Discord(DiscordConfig {
                        webhook_url: notifier_value.to_string(),
                    }),
                    _ => continue,
                };

                let message = format!(
                    "Alert: {}\nDescription: {}\nCondition met on system ID {}",
                    rule.name, rule.description, system_id
                );

                if let Err(e) = service.send(&message).await {
                    eprintln!("Failed to send notification: {}", e);
                } else {
                    info!("Notification sent via {}", notifier_type);
                }
            }
            // insert into alert_history (system, alert, date)
            sqlx::query(
                r#"
INSERT INTO alert_history (system, alert, date)
VALUES ($1, $2, NOW())
"#,
            )
            .bind(system_id)
            .bind(rule.id.parse::<i32>()?)
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}
