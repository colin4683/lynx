use crate::proto::monitor::{ContainerMetrics, ContainerMetricsRequest, MetricsRequest};
use chrono::{DateTime, Utc};
use log::{error, info};
use sqlx::{PgPool, QueryBuilder};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};
use tonic::Status;

#[derive(Debug)]
pub struct DiskEntry {
    pub name: String,
    pub total_space: i64,
    pub used_space: i64,
    pub read_bytes: f64,
    pub write_bytes: f64,
    pub unit: String,
    pub mount_point: String,
}

#[derive(Debug)]
pub struct MetricIngestItem {
    pub system_id: i32,
    pub time: DateTime<Utc>,
    pub cpu_usage: f64,
    pub memory_used_kb: i64,
    pub memory_total_kb: i64,
    pub components_json: String,
    pub net_in: i64,
    pub net_out: i64,
    pub load_one: f64,
    pub load_five: f64,
    pub load_fifteen: f64,
    pub disks: Vec<DiskEntry>,
    pub original: MetricsRequest, // for notifications
}

#[derive(Debug)]
pub struct ContainerIngestItem {
    pub system_id: i32,
    pub time: DateTime<Utc>,
    pub docker_id: String,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub original: ContainerMetrics, // for notifications
}

#[derive(Debug)]
pub enum IngestItem {
    Metric(MetricIngestItem),
    Container(ContainerIngestItem),
}

#[derive(Debug)]
pub struct MetricWorkerState {
    last_alert_check: Instant,
    active_alerts: Arc<RwLock<HashSet<(String)>>>,
}

const METRIC_BATCH_MAX: usize = 200;
const METRIC_FLUSH_MS: u64 = 3000;

const ALERT_COOLDOWN: Duration = Duration::from_secs(600); // 10 minutes

pub async fn run_metric_worker(mut rx: Receiver<IngestItem>, pool: PgPool) {
    use tokio::time::{timeout, Duration};

    let mut batch: Vec<IngestItem> = Vec::with_capacity(METRIC_BATCH_MAX);
    let mut last_flush = Instant::now();
    let alert_history = Arc::new(RwLock::new(HashMap::<String, Instant>::new()));
    loop {
        // Ensure at least one item (or exit if channel is closed)
        if batch.is_empty() {
            match rx.recv().await {
                Some(item) => {
                    batch.push(item);
                    last_flush = Instant::now();
                }
                None => break,
            }
        }

        // Fill batch until size or timeout
        while batch.len() < METRIC_BATCH_MAX {
            let elapsed = last_flush.elapsed();
            let remaining = if elapsed.as_millis() as u64 >= METRIC_FLUSH_MS {
                Duration::from_millis(0)
            } else {
                Duration::from_millis(METRIC_FLUSH_MS - elapsed.as_millis() as u64)
            };

            match timeout(remaining, rx.recv()).await {
                Ok(Some(item)) => batch.push(item),
                Ok(None) => {
                    // Sender dropped; flush what we have then exit
                    break;
                }
                Err(_) => {
                    // timeout reached
                    break;
                }
            }
        }

        if !batch.is_empty() {
            if let Err(e) = flush_batch(&pool, &batch).await {
                error!("[ingest] Batch flush failed: {e}");
            } else {
                let pool_clone = pool.clone();
                let state_clone = alert_history.clone();

                // Currently only processing notifications for MetricIngestItem
                // todo: Add support for container metrics notifications
                if let IngestItem::Container(_) = batch[0] {
                    batch.clear();
                    continue;
                }

                let batch_clone: Vec<_> = batch
                    .iter()
                    .filter_map(|item| {
                        if let IngestItem::Metric(m) = item {
                            Some((m.system_id, m.original.clone()))
                        } else {
                            None
                        }
                    })
                    .collect();

                cleanup_expired_alerts(&alert_history, ALERT_COOLDOWN).await;

                tokio::spawn(async move {
                    process_batch_notifications(&pool_clone, &batch_clone, &state_clone).await;
                });
            }
            batch.clear();
        }

        // Exit if channel closed and nothing pending
        if rx.is_closed() && batch.is_empty() {
            break;
        }
    }

    info!("[ingest] Metric worker stopped");
}

async fn flush_batch(pool: &PgPool, batch: &[IngestItem]) -> Result<(), sqlx::Error> {
    if batch.is_empty() {
        return Ok(());
    }

    let mut tx = pool.begin().await?;
    match batch {
        [IngestItem::Metric(_), ..] => {
            let metrics: Vec<&MetricIngestItem> = batch
                .iter()
                .filter_map(|item| {
                    if let IngestItem::Metric(m) = item {
                        Some(m)
                    } else {
                        None
                    }
                })
                .collect();
            {
                let mut qb = QueryBuilder::new(
                    "INSERT INTO metrics (time, system_id, cpu_usage, memory_used_kb, memory_total_kb, components, net_in, net_out, load_one, load_five, load_fifteen) ",
                );
                qb.push_values(metrics.iter(), |mut b, m| {
                    b.push_bind(m.time)
                        .push_bind(m.system_id)
                        .push_bind(m.cpu_usage)
                        .push_bind(m.memory_used_kb)
                        .push_bind(m.memory_total_kb)
                        .push_bind(&m.components_json)
                        .push_bind(m.net_in)
                        .push_bind(m.net_out)
                        .push_bind(m.load_one)
                        .push_bind(m.load_five)
                        .push_bind(m.load_fifteen);
                });
                qb.build().execute(&mut *tx).await?;
            }

            // Gather all disks
            let mut latest_disks: HashMap<(i32, &str), (&DiskEntry, i32)> = HashMap::new();
            for m in metrics {
                for d in &m.disks {
                    latest_disks.insert((m.system_id, d.name.as_str()), (d, m.system_id));
                }
            }

            if !latest_disks.is_empty() {
                let mut qb = QueryBuilder::new(
                    "INSERT INTO disks \
     (system, name, unit, mount_point, space, used, read, write, time) ",
                );

                let disks: Vec<&DiskEntry> = latest_disks.values().map(|(d, _)| *d).collect();
                let system_id = latest_disks.values().next().unwrap().1; // all have
                let now = chrono::Utc::now();
                qb.push_values(disks.iter(), |mut b, disk| {
                    b.push_bind(system_id) // i64
                        .push_bind(&disk.name) // String
                        .push_bind(&disk.unit)
                        .push_bind(&disk.mount_point)
                        .push_bind(disk.total_space) // i64
                        .push_bind(disk.used_space) // i64
                        .push_bind(disk.read_bytes) // f64
                        .push_bind(disk.write_bytes) // f64
                        .push_bind(now); // Timestamp
                });

                qb.push(
                    " ON CONFLICT (system, name, time) DO UPDATE SET \
              unit = EXCLUDED.unit, \
              mount_point = EXCLUDED.mount_point, \
              space = EXCLUDED.space, \
              used = EXCLUDED.used, \
              read = EXCLUDED.read, \
              write = EXCLUDED.write, \
              time = NOW()",
                );

                qb.build().execute(&mut *tx).await?;
            }
        }
        [IngestItem::Container(_), ..] => {
            let containers: Vec<&ContainerIngestItem> = batch
                .iter()
                .filter_map(|item| {
                    if let IngestItem::Container(c) = item {
                        Some(c)
                    } else {
                        None
                    }
                })
                .collect();
            if !containers.is_empty() {
                // Collect owned Strings to match expected &\[String]
                let ids: Vec<String> = containers.iter().map(|m| m.docker_id.clone()).collect();

                let rows = sqlx::query!(
                    "SELECT id, docker_id FROM containers WHERE system_id = $1 AND docker_id = ANY($2)",
                    containers[0].system_id,
                    &ids
                    )
                    .fetch_all(pool)
                    .await?;

                let mut id_map = std::collections::HashMap::new();
                for r in rows {
                    id_map.insert(r.docker_id, r.id);
                }

                let mut qb = QueryBuilder::new(
                    "INSERT INTO container_metrics (container_id, time, cpu_usage, memory_usage) ",
                );
                let now = Utc::now();
                let mut any = false;
                qb.push_values(
                    containers
                        .iter()
                        .filter_map(|m| id_map.get(&m.docker_id).map(|cid| (cid, m))),
                    |mut b, (cid, m)| {
                        any = true;
                        b.push_bind(*cid)
                            .push_bind(now)
                            .push_bind(m.cpu_usage)
                            .push_bind(m.memory_usage);
                    },
                );
                if !any {
                    return Ok(());
                }
                qb.build().execute(&mut *tx).await?;
            }
        }
        _ => {}
    }
    tx.commit().await?;
    info!("[ingest] Flushed {} items", batch.len());
    Ok(())
}

async fn cleanup_expired_alerts(state: &Arc<RwLock<HashMap<String, Instant>>>, cooldown: Duration) {
    let mut alerts = state.write().await;
    let now = Instant::now();
    alerts.retain(|_, &mut last_triggered| now.duration_since(last_triggered) < cooldown);
}

async fn process_batch_notifications(
    pool: &PgPool,
    batch: &[(i32, MetricsRequest)],
    triggered_alerts: &Arc<RwLock<HashMap<String, Instant>>>,
) {
    for (system_id, metrics) in batch {
        let active_alerts = {
            let alerts = triggered_alerts.read().await;
            alerts.keys().cloned().collect::<HashSet<String>>()
        };

        match crate::notify::process_notification(metrics, *system_id, pool, &active_alerts).await {
            Ok(new_triggered) => {
                if !new_triggered.is_empty() {
                    let mut alerts = triggered_alerts.write().await;
                    let now = Instant::now();
                    for rule_name in new_triggered {
                        alerts.insert(rule_name, now);
                    }
                    info!("[notify] System {}: Alerts Updated", system_id);
                }
            }
            Err(e) => error!("[notify] Failed for system {}: {e}", system_id),
        }
    }
}
