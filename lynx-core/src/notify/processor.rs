use super::*;
use crate::proto::monitor::MetricsRequest;
use log::{error, info, warn};
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct NotificationProcessor {
    registry: MetricRegistry,
    services: Arc<Mutex<HashMap<String, NotificationServiceType>>>,
    pool: PgPool,
}

impl NotificationProcessor {
    pub fn new(pool: PgPool) -> Self {
        Self {
            registry: MetricRegistry::new(),
            services: Arc::new(Mutex::new(HashMap::new())),
            pool,
        }
    }

    /*
     * register_metrics
     * Registers available metric components used for alert logic based on the incoming
     * MetricsRequest.
     */
    pub async fn register_metrics(&self, metrics: &MetricsRequest) {
        if let Some(cpu_stats) = &metrics.cpu_stats {
            self.registry
                .register_component(
                    "cpu".to_string(),
                    Box::new(CpuComponent::new(cpu_stats.clone())),
                )
                .await;
        }

        if let Some(memory_stats) = &metrics.memory_stats {
            self.registry
                .register_component(
                    "memory".to_string(),
                    Box::new(MemoryComponent::new(memory_stats.clone())),
                )
                .await;
        }

        if let Some(load_avg) = &metrics.load_average {
            self.registry
                .register_component(
                    "load".to_string(),
                    Box::new(LoadComponent::new(load_avg.clone())),
                )
                .await;
        }

        if !metrics.disk_stats.is_empty() {
            self.registry
                .register_component(
                    "disk".to_string(),
                    Box::new(DiskComponent::new(metrics.disk_stats.clone())),
                )
                .await;
        }

        if let Some(network_stats) = &metrics.network_stats {
            self.registry
                .register_component(
                    "network".to_string(),
                    Box::new(NetworkComponent::new(network_stats.clone())),
                )
                .await;
        }
    }

    /*
     * load_rules
     * Combines alert rules with their associated notifiers from the database for a given system.
     */
    async fn load_rules(&self, system_id: i32) -> Result<Vec<(Rule, Vec<String>)>, sqlx::Error> {
        let alerts = sqlx::query(crate::queries::alert_queries::GET_ALERT_SYSTEMS)
            .bind(system_id)
            .fetch_all(&self.pool)
            .await?;

        let mut rules_with_notifiers = Vec::new();

        for alert in alerts {
            let rule_id: i32 = alert.get("rule_id");
            let row = sqlx::query(crate::queries::alert_queries::GET_ALERT_RULES)
                .bind(rule_id)
                .fetch_one(&self.pool)
                .await?;

            let name: String = row.get("name");
            let enabled: bool = row.get("active");
            let expression: String = row.get("expression");
            let severity: String = row.get("severity");
            let description: String = row.get("description");

            // Parse the rule expression
            let conditions = match RuleParser::parse_expression(&expression) {
                Ok(conditions) => conditions,
                Err(e) => {
                    warn!("Failed to parse rule {}: {}", name, e);
                    continue;
                }
            };

            let rule = Rule {
                id: rule_id,
                name,
                enabled,
                description,
                severity,
                conditions,
            };

            // Get notifiers for this rule
            let notifiers = sqlx::query(crate::queries::alert_queries::GET_ALERT_NOTIFIERS)
                .bind(rule_id)
                .fetch_all(&self.pool)
                .await?;

            let mut notifier_urls = Vec::new();
            for notifier in notifiers {
                let notifier_id: i32 = notifier.get("notifier_id");
                let notifier_row = sqlx::query(crate::queries::alert_queries::GET_NOTIFIERS)
                    .bind(notifier_id)
                    .fetch_one(&self.pool)
                    .await?;

                let notifier_type: String = notifier_row.get("type");
                let notifier_value: String = notifier_row.get("value");
                notifier_urls.push(format!("{}", notifier_value));
            }

            rules_with_notifiers.push((rule, notifier_urls));
        }

        Ok(rules_with_notifiers)
    }

    /*
     * get_or_create_service
     * Retrieves an existing notification service or creates a new one based on the provided URL.
     */
    async fn get_or_create_service(
        &self,
        url: &str,
    ) -> Result<NotificationServiceType, NotificationError> {
        let mut services = self.services.lock().await;

        if !services.contains_key(url) {
            let service = NotificationServiceType::from_url(url)?;
            services.insert(url.to_string(), service);
        }

        Ok(services.get(url).unwrap().clone())
    }

    /*
     * notify::processor::process
     * Processes metrics for a given system, evaluates rules, and sends notifications if rules
     * are triggered. Called after metrics are ingested and inserted into the database.
     */
    pub async fn process(
        &self,
        metrics: &MetricsRequest,
        system_id: i32,
        triggered_rules: &HashSet<String>,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send>> {
        // Register metrics from the request
        self.register_metrics(metrics).await;

        let rules = self
            .load_rules(system_id)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;
        let evaluator = RuleEvaluator::new(&self.registry);
        let mut triggerd_rules = Vec::new();
        for (rule, notifier_urls) in rules {
            if !rule.enabled {
                continue;
            }

            // Skip already triggered rules in the current context
            if triggered_rules.contains(&rule.name) {
                continue;
            }

            match evaluator.evaluate_rule(&rule).await {
                Ok(true) => {
                    info!("Rule '{}' triggered for system {}", rule.name, system_id);

                    // Insert alert history
                    if let Err(e) = sqlx::query(crate::queries::alert_queries::INSERT_ALERT_HISTORY)
                        .bind(system_id)
                        .bind(rule.id)
                        .execute(&self.pool)
                        .await
                    {
                        error!("Failed to insert alert history: {}", e);
                    }

                    // Send notifications
                    let message = format!(
                        "Alert: {}\nDescription: {}\nSeverity: {}\nSystem ID: {}",
                        rule.name, rule.description, rule.severity, system_id
                    );

                    for url in notifier_urls {
                        match self.get_or_create_service(&url).await {
                            Ok(service) => {
                                if let Err(e) = service.send(&message).await {
                                    error!("Failed to send notification via {}: {}", url, e);
                                }
                            }
                            Err(e) => {
                                error!("Failed to create notification service for {}: {}", url, e);
                            }
                        }
                    }
                    triggerd_rules.push(rule.name.clone());
                }
                Ok(false) => {}
                Err(e) => {
                    warn!("Failed to evaluate rule '{}': {}", rule.name, e);
                }
            }
        }
        Ok(triggerd_rules)
    }
}
