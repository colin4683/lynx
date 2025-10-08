use chrono::{DateTime, Utc};
use log::{error, info};
use sqlx::{PgPool, QueryBuilder};
use std::collections::HashMap;
use tokio::sync::mpsc::Receiver;

use crate::proto::monitor::MetricsRequest;

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

const METRIC_BATCH_MAX: usize = 200;
const METRIC_FLUSH_MS: u64 = 10000;

pub async fn run_metric_worker(mut rx: Receiver<MetricIngestItem>, pool: PgPool) {
    use std::time::Instant;
    use tokio::time::{timeout, Duration};

    let mut batch: Vec<MetricIngestItem> = Vec::with_capacity(METRIC_BATCH_MAX);
    let mut last_flush = Instant::now();

    loop {
        // Ensure at least one item (or exit if channel closed)
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
                // spawn notifications after successful persistence
                for item in &batch {
                    let pool_clone = pool.clone();
                    let metrics_clone = item.original.clone();
                    let system_id = item.system_id;
                    tokio::spawn(async move {
                        if let Err(err) = crate::notify::process_notification(
                            &metrics_clone,
                            system_id as i32,
                            &pool_clone,
                        )
                        .await
                        {
                            error!("[notify] Failed: {err}");
                        }
                    });
                }
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

async fn flush_batch(pool: &PgPool, batch: &[MetricIngestItem]) -> Result<(), sqlx::Error> {
    if batch.is_empty() {
        return Ok(());
    }

    let mut tx = pool.begin().await?;

    // Metrics multi-row
    {
        let mut qb = QueryBuilder::new(
            "INSERT INTO metrics (time, system_id, cpu_usage, memory_used_kb, memory_total_kb, components, net_in, net_out, load_one, load_five, load_fifteen) ",
        );
        qb.push_values(batch.iter(), |mut b, m| {
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
    for m in batch {
        for d in &m.disks {
            latest_disks.insert((m.system_id, d.name.as_str()), (d, m.system_id));
        }
    }

    /*if !latest_disks.is_empty() {
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
            " ON CONFLICT (system, name) DO UPDATE SET \
              unit = EXCLUDED.unit, \
              mount_point = EXCLUDED.mount_point, \
              space = EXCLUDED.space, \
              used = EXCLUDED.used, \
              read = EXCLUDED.read, \
              write = EXCLUDED.write, \
              time = NOW()",
        );

        qb.build().execute(pool).await?;
    }*/

    tx.commit().await?;
    Ok(())
}
