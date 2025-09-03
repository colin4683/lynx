// Re-exported notification logic moved from src/lib/notify.rs to avoid module path conflicts.
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
                match action.as_str() {
                    "low" => {
                        let discord_config = DiscordConfig {
                            webhook_url: "https://discord.example/webhook".to_string(),
                        };
                        let message =
                            format!("Low Severity Alert: {} - {}", rule.name, rule.description);
                        let _ = NotificationService::Discord(discord_config)
                            .send(&message)
                            .await;
                    }
                    _ => {}
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
            NotificationService::Email(_config) => Ok(()),
            NotificationService::Discord(config) => send_discord(config, message).await,
            NotificationService::Slack(_config) => Ok(()),
        }
    }
}

async fn send_discord(config: &DiscordConfig, message: &str) -> Result<(), NotificationError> {
    let client = Client::new();
    let _res = client
        .post(&config.webhook_url)
        .json(&serde_json::json!({ "content": message }))
        .send()
        .await?;
    Ok(())
}
