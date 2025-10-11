use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

pub mod components;
pub mod processor;
pub mod rules;
pub mod services;

pub use components::*;
pub use processor::*;
pub use rules::*;
pub use services::*;

/*
 * Notification System
 * This system provides a modular way to process alerts. It works by generating a registry of
 * available metric components from the given metric request. These could be cpu, memory, disk,
 * network, etc. Each component implements the MetricComponent trait, which binds the "metrics"
 * of each component to that component. For example, the CPU component may have metrics like
 * "usage", "temperature", etc. The Memory component may have "used", "total", "usage", etc. Once
 * a new metric request is received, the registry is populated with the available components.
 * Then the alert rules are retrieved for the given system. Each rule is evaluated using the
 * registry to fetch the necessary metric values. If a rule triggers, the associated notifier
 * for that rule gets executed.
 */

#[derive(Error, Debug)]
pub enum MetricError {
    #[error("Component not found: {0}")]
    ComponentNotFound(String),
    #[error("Metric not found: {0}")]
    MetricNotFound(String),
    #[error("Invalid value: {0}")]
    InvalidValue(String),
}

#[async_trait]
pub trait MetricComponent: Send + Sync {
    async fn get_metric(&self, metric_name: &str) -> Result<f64, MetricError>;
    fn available_metrics(&self) -> Vec<&str>;
}

#[async_trait]
pub trait NotificationService: Send + Sync + Clone {
    async fn send(&self, message: &str) -> Result<(), NotificationError>;
}

pub struct MetricRegistry {
    components: Arc<RwLock<HashMap<String, Box<dyn MetricComponent>>>>,
}

impl MetricRegistry {
    pub fn new() -> Self {
        Self {
            components: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_component(&self, name: String, component: Box<dyn MetricComponent>) {
        let mut components = self.components.write().await;
        components.insert(name, component);
    }

    pub async fn get_metric_value(
        &self,
        component: &str,
        metric: &str,
    ) -> Result<f64, MetricError> {
        let components = self.components.read().await;
        if let Some(comp) = components.get(component) {
            comp.get_metric(metric).await
        } else {
            Err(MetricError::ComponentNotFound(component.to_string()))
        }
    }
}

use crate::proto::monitor::MetricsRequest;
use sqlx::PgPool;

/*
 * process_notification
 * Main entry point to process notifications for a given MetricsRequest
 */
pub async fn process_notification(
    metrics: &MetricsRequest,
    system_id: i32,
    pool: &PgPool,
    triggered_rules: &HashSet<String>,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send>> {
    let mut processor = NotificationProcessor::new(pool.clone());
    processor.process(metrics, system_id, triggered_rules).await
}
