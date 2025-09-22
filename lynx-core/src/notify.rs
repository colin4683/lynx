use async_trait::async_trait;
use std::collections::HashMap;
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

// Custom error type for metric evaluation
#[derive(Error, Debug)]
pub enum MetricError {
    #[error("Component not found: {0}")]
    ComponentNotFound(String),
    #[error("Metric not found: {0}")]
    MetricNotFound(String),
    #[error("Invalid value: {0}")]
    InvalidValue(String),
}

// Core traits
#[async_trait]
pub trait MetricComponent: Send + Sync {
    async fn get_metric(&self, metric_name: &str) -> Result<f64, MetricError>;
    fn available_metrics(&self) -> Vec<&str>;
}

#[async_trait]
pub trait NotificationService: Send + Sync + Clone {
    async fn send(&self, message: &str) -> Result<(), NotificationError>;
}

// Thread-safe metric registry
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

/// Process notifications for a system's metrics
///
/// This function is the main entry point for the notification system.
/// It handles metric processing, rule evaluation, and notification dispatch
/// in a modular and fault-tolerant way.
pub async fn process_notification(
    metrics: &MetricsRequest,
    system_id: i32,
    pool: &PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut processor = NotificationProcessor::new(pool.clone());
    processor.process(metrics, system_id).await
}
