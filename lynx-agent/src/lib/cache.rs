use bincode::error::{DecodeError, EncodeError};
use bincode::{config, Decode, Encode};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use log::info;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePool, Row};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Serialization encode error: {0}")]
    Encode(#[from] EncodeError),
    #[error("Serialization decode error: {0}")]
    Decode(#[from] DecodeError),
    #[error("Entry not found: {0}")]
    NotFound(String),
    #[error("Invalid key: {0}")]
    InvalidKey(String),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub id: Uuid,
    pub key: String,
    pub value: T,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode, PartialEq)]
pub struct SystemService {
    pub name: String,
    pub status: String,
    pub enabled: bool,
    pub description: Option<String>,
    pub pid: Option<u64>,
    pub cpu_usage: Option<String>,
    pub memory_usage: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct ConfigChange {
    pub file_path: String,
    pub change_type: String, // "created", "modified", "deleted"
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub user: Option<String>,
    pub checksum: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct LogEntry {
    pub level: String,
    pub message: String,
    pub source: String,
    pub metadata: std::collections::HashMap<String, String>,
}

pub type CacheResult<T> = Result<T, CacheError>;

pub struct FastCache {
    // In-memory cache for ultra-fast access
    memory_cache: Arc<DashMap<String, Vec<u8>>>,
    // Metadata cache for quick lookups
    metadata_cache: Arc<DashMap<String, CacheMetadata>>,
    // SQLite for persistence
    db_pool: SqlitePool,
    // Write-through vs write-back mode
    write_through: bool,
}

#[derive(Debug, Clone)]
struct CacheMetadata {
    expires_at: Option<DateTime<Utc>>,
    tags: Vec<String>,
    size: usize,
}

impl FastCache {
    pub async fn new(database_url: &str, write_through: bool) -> CacheResult<Self> {
        let db_pool = SqlitePool::connect(database_url).await?;

        // Create tables
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS cache_entries (
                key TEXT PRIMARY KEY,
                value BLOB NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                expires_at TEXT,
                tags TEXT
            )
            "#,
        )
        .execute(&db_pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_cache_expires_at ON cache_entries(expires_at);
            CREATE INDEX IF NOT EXISTS idx_cache_tags ON cache_entries(tags);
            CREATE INDEX IF NOT EXISTS idx_cache_updated_at ON cache_entries(updated_at);
            "#,
        )
        .execute(&db_pool)
        .await?;

        let cache = Self {
            memory_cache: Arc::new(DashMap::new()),
            metadata_cache: Arc::new(DashMap::new()),
            db_pool,
            write_through,
        };

        // Load existing data into memory cache on startup
        cache.load_from_disk().await?;

        Ok(cache)
    }

    async fn load_from_disk(&self) -> CacheResult<()> {
        let rows = sqlx::query("SELECT key, value, expires_at, tags FROM cache_entries")
            .fetch_all(&self.db_pool)
            .await?;

        for row in rows {
            let key: String = row.get("key");
            let value: Vec<u8> = row.get("value");
            let expires_at: Option<String> = row.get("expires_at");
            let tags: Option<String> = row.get("tags");

            let expires_at = expires_at
                .map(|s| DateTime::parse_from_rfc3339(&s))
                .transpose()
                .map_err(|_| CacheError::InvalidKey("Invalid expires_at format".to_string()))?
                .map(|dt| dt.with_timezone(&Utc));

            // Skip expired entries
            if let Some(exp) = expires_at {
                if exp < Utc::now() {
                    continue;
                }
            }

            let parsed_tags: Vec<String> = tags
                .map(|t| serde_json::from_str(&t).unwrap_or_default())
                .unwrap_or_default();

            self.memory_cache.insert(key.clone(), value.clone());
            self.metadata_cache.insert(
                key,
                CacheMetadata {
                    expires_at,
                    tags: parsed_tags,
                    size: value.len(),
                },
            );
        }

        Ok(())
    }

    pub async fn set<T>(
        &self,
        key: &str,
        value: &T,
        ttl: Option<chrono::Duration>,
        tags: Vec<String>,
    ) -> CacheResult<()>
    where
        T: Encode,
    {
        let serialized =
            bincode::encode_to_vec(value, config::standard()).map_err(CacheError::Encode)?;
        let now = Utc::now();
        let expires_at = ttl.map(|duration| now + duration);

        // Store in memory cache
        self.memory_cache
            .insert(key.to_string(), serialized.clone());
        self.metadata_cache.insert(
            key.to_string(),
            CacheMetadata {
                expires_at,
                tags: tags.clone(),
                size: serialized.len(),
            },
        );

        // Persist to disk
        if self.write_through {
            self.persist_to_disk(key, &serialized, now, expires_at, &tags)
                .await?;
        }

        Ok(())
    }

    pub async fn get<T>(&self, key: &str) -> CacheResult<Option<T>>
    where
        T: Decode<()>,
    {
        // Check if expired
        if let Some(metadata) = self.metadata_cache.get(key) {
            if let Some(expires_at) = metadata.expires_at {
                if expires_at < Utc::now() {
                    self.delete(key).await?;
                    return Ok(None);
                }
            }
        }

        // Get from memory cache
        if let Some(data) = self.memory_cache.get(key) {
            let (value, _): (T, usize) = bincode::decode_from_slice(&data, config::standard())
                .map_err(CacheError::Decode)?;
            return Ok(Some(value));
        }

        Ok(None)
    }

    pub async fn delete(&self, key: &str) -> CacheResult<bool> {
        let existed = self.memory_cache.remove(key).is_some();
        self.metadata_cache.remove(key);

        if self.write_through {
            sqlx::query("DELETE FROM cache_entries WHERE key = ?")
                .bind(key)
                .execute(&self.db_pool)
                .await?;
        }

        Ok(existed)
    }

    pub async fn get_by_tags(&self, tags: &[String]) -> CacheResult<Vec<String>> {
        let mut matching_keys = Vec::new();

        for entry in self.metadata_cache.iter() {
            let key = entry.key();
            let metadata = entry.value();

            // Check if expired
            if let Some(expires_at) = metadata.expires_at {
                if expires_at < Utc::now() {
                    continue;
                }
            }

            // Check if any of the requested tags match
            if tags.iter().any(|tag| metadata.tags.contains(tag)) {
                matching_keys.push(key.clone());
            }
        }

        Ok(matching_keys)
    }

    pub async fn clear_expired(&self) -> CacheResult<usize> {
        info!("[cache] Running cleanup of expired cache entries");
        let now = Utc::now();
        let mut expired_keys = Vec::new();

        for entry in self.metadata_cache.iter() {
            if let Some(expires_at) = entry.value().expires_at {
                if expires_at < now {
                    expired_keys.push(entry.key().clone());
                }
            }
        }

        let count = expired_keys.len();
        for key in expired_keys {
            self.delete(&key).await?;
        }
        info!("[cache] Cleaned up {} expired cache entries", count);

        Ok(count)
    }

    pub fn cache_stats(&self) -> CacheStats {
        let total_entries = self.memory_cache.len();
        let total_size: usize = self
            .metadata_cache
            .iter()
            .map(|entry| entry.value().size)
            .sum();

        CacheStats {
            total_entries,
            total_size_bytes: total_size,
            memory_entries: total_entries,
        }
    }

    async fn persist_to_disk(
        &self,
        key: &str,
        value: &[u8],
        created_at: DateTime<Utc>,
        expires_at: Option<DateTime<Utc>>,
        tags: &[String],
    ) -> CacheResult<()> {
        let expires_at_str = expires_at.map(|dt| dt.to_rfc3339());
        let tags_json = serde_json::to_string(tags).unwrap_or_default();

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO cache_entries (key, value, created_at, updated_at, expires_at, tags)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(key)
        .bind(value)
        .bind(created_at.to_rfc3339())
        .bind(created_at.to_rfc3339())
        .bind(expires_at_str)
        .bind(tags_json)
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }

    // Batch operations for better performance
    pub async fn set_batch<T>(
        &self,
        entries: Vec<(String, T, Option<chrono::Duration>, Vec<String>)>,
    ) -> CacheResult<()>
    where
        T: Encode,
    {
        for (key, value, ttl, tags) in entries {
            self.set(&key, &value, ttl, tags).await?;
        }
        Ok(())
    }

    pub async fn flush_to_disk(&self) -> CacheResult<()> {
        if self.write_through {
            return Ok(()); // Already synced
        }

        let now = Utc::now();
        for entry in self.memory_cache.iter() {
            let key = entry.key();
            let value = entry.value();

            if let Some(metadata) = self.metadata_cache.get(key) {
                let expires_at = metadata.expires_at;
                let tags = &metadata.tags;

                self.persist_to_disk(key, value, now, expires_at, tags)
                    .await?;
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_size_bytes: usize,
    pub memory_entries: usize,
}

// Convenience methods for specific data types
impl FastCache {
    pub async fn set_system_service(
        &self,
        service: &SystemService,
        ttl: Option<chrono::Duration>,
    ) -> CacheResult<()> {
        let key = format!("service:{}", service.name);
        self.set(
            &key,
            service,
            ttl,
            vec!["system".to_string(), "service".to_string()],
        )
        .await
    }

    pub async fn get_system_service(&self, name: &str) -> CacheResult<Option<SystemService>> {
        let key = format!("service:{}", name);
        self.get(&key).await
    }

    pub async fn set_config_change(&self, change: &ConfigChange) -> CacheResult<()> {
        let key = format!("config:{}:{}", change.file_path, Utc::now().timestamp());
        self.set(
            &key,
            change,
            Some(chrono::Duration::days(30)),
            vec!["config".to_string(), "change".to_string()],
        )
        .await
    }

    pub async fn set_log_entry(&self, entry: &LogEntry) -> CacheResult<()> {
        let key = format!("log:{}:{}", entry.source, Uuid::new_v4());
        self.set(
            &key,
            entry,
            Some(chrono::Duration::days(7)),
            vec!["log".to_string(), entry.level.clone()],
        )
        .await
    }

    pub async fn get_services(&self) -> CacheResult<Vec<SystemService>> {
        let keys = self.get_by_tags(&["service".to_string()]).await?;
        let mut services = Vec::new();

        for key in keys {
            if let Ok(Some(service)) = self.get::<SystemService>(&key).await {
                services.push(service);
            }
        }

        Ok(services)
    }

    pub async fn get_config_changes(&self) -> CacheResult<Vec<ConfigChange>> {
        let keys = self.get_by_tags(&["config".to_string()]).await?;
        let mut changes = Vec::new();

        for key in keys {
            if let Ok(Some(change)) = self.get::<ConfigChange>(&key).await {
                changes.push(change);
            }
        }

        Ok(changes)
    }

    pub async fn get_logs_by_level(&self, level: &str) -> CacheResult<Vec<LogEntry>> {
        let keys = self.get_by_tags(&[level.to_string()]).await?;
        let mut logs = Vec::new();

        for key in keys {
            if let Ok(Some(log)) = self.get::<LogEntry>(&key).await {
                logs.push(log);
            }
        }

        Ok(logs)
    }

    pub async fn sync_from_disk(&self) -> CacheResult<()> {
        // Clear current in-memory state
        self.memory_cache.clear();
        self.metadata_cache.clear();
        // Reload from disk
        self.load_from_disk().await
    }
}

// Background cleanup task
pub async fn start_cleanup_task(cache: Arc<FastCache>, interval: Duration) {
    let mut interval_timer = tokio::time::interval(Duration::from_secs(interval.as_secs()));

    loop {
        interval_timer.tick().await;

        if let Err(e) = cache.clear_expired().await {
            log::error!("Error during cache cleanup: {}", e);
        } else {
            log::debug!("Cache cleanup completed");
        }
    }
}
