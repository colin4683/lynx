use chrono::{DateTime, Utc};
use dashmap::DashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::proto::monitor::SystemService;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfigChange {
    pub key: String,
    pub old_value: Option<String>,
    pub new_value: String,
    pub ts: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub level: String,
    pub message: String,
    pub ts: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SerializableSystemService {
    pub service_name: String,
    pub description: String,
    pub pid: u64,
    pub state: String,
    pub cpu: String,
    pub memory: String,
}

impl From<&SystemService> for SerializableSystemService {
    fn from(s: &SystemService) -> Self {
        Self {
            service_name: s.service_name.clone(),
            description: s.description.clone(),
            pid: s.pid,
            state: s.state.clone(),
            cpu: s.cpu.clone(),
            memory: s.memory.clone(),
        }
    }
}

impl From<SerializableSystemService> for SystemService {
    fn from(s: SerializableSystemService) -> Self {
        Self {
            service_name: s.service_name,
            description: s.description,
            pid: s.pid,
            state: s.state,
            cpu: s.cpu,
            memory: s.memory,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct CacheSnapshot {
    services: Vec<SerializableSystemService>,
    config_changes: Vec<ConfigChange>,
    logs: Vec<LogEntry>,
}

#[derive(Clone)]
pub struct Cache {
    services: Arc<DashMap<String, SystemService>>,
    config_changes: Arc<RwLock<Vec<ConfigChange>>>,
    logs: Arc<RwLock<Vec<LogEntry>>>,
    max_logs: usize,
    max_config_changes: usize,
}

impl Cache {
    pub fn new(max_logs: usize, max_config_changes: usize) -> Self {
        Self {
            services: Arc::new(DashMap::new()),
            config_changes: Arc::new(RwLock::new(Vec::new())),
            logs: Arc::new(RwLock::new(Vec::new())),
            max_logs,
            max_config_changes,
        }
    }

    pub fn upsert_service(&self, svc: SystemService) {
        self.services.insert(svc.service_name.clone(), svc);
    }

    pub fn get_service(&self, name: &str) -> Option<SystemService> {
        self.services.get(name).map(|s| s.clone())
    }

    pub fn list_services(&self) -> Vec<SystemService> {
        self.services.iter().map(|r| r.clone()).collect()
    }

    pub async fn record_config_change(
        &self,
        key: String,
        old_value: Option<String>,
        new_value: String,
    ) {
        let mut guard = self.config_changes.write().await;
        guard.push(ConfigChange {
            key,
            old_value,
            new_value,
            ts: Utc::now(),
        });
        if guard.len() > self.max_config_changes {
            let overflow = guard.len() - self.max_config_changes;
            guard.drain(0..overflow);
        }
    }

    pub async fn record_log(&self, level: impl Into<String>, message: impl Into<String>) {
        let mut guard = self.logs.write().await;
        guard.push(LogEntry {
            level: level.into(),
            message: message.into(),
            ts: Utc::now(),
        });
        if guard.len() > self.max_logs {
            let overflow = guard.len() - self.max_logs;
            guard.drain(0..overflow);
        }
    }

    pub async fn snapshot_to_file(&self, path: &Path) -> std::io::Result<()> {
        let services: Vec<SerializableSystemService> =
            self.list_services().iter().map(|s| s.into()).collect();
        let config_changes = self.config_changes.read().await.clone();
        let logs = self.logs.read().await.clone();
        let snap = CacheSnapshot {
            services,
            config_changes,
            logs,
        };
        let bytes = bincode::serialize(&snap)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        tokio::fs::write(path, bytes).await
    }

    pub async fn load_from_file(&self, path: &Path) -> std::io::Result<()> {
        if !path.exists() {
            return Ok(());
        }
        let bytes = tokio::fs::read(path).await?;
        let snap: CacheSnapshot = bincode::deserialize(&bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        for svc in snap.services {
            self.upsert_service(SystemService::from(svc));
        }
        {
            let mut cfg = self.config_changes.write().await;
            *cfg = snap.config_changes;
        }
        {
            let mut lg = self.logs.write().await;
            *lg = snap.logs;
        }
        Ok(())
    }

    pub fn service_count(&self) -> usize {
        self.services.len()
    }

    pub async fn log_count(&self) -> usize {
        self.logs.read().await.len()
    }

    pub async fn config_change_count(&self) -> usize {
        self.config_changes.read().await.len()
    }
}
