use crate::lib;
use crate::lib::cache::FastCache;
use crate::proto::monitor::{MetricsRequest, SystemInfoRequest, SystemctlRequest};
use log::info;
use std::sync::Arc;
use std::time::Duration;
use sysinfo::{System, MINIMUM_CPU_UPDATE_INTERVAL};
use tokio::sync::mpsc;
use tokio::time::{timeout, Instant};

pub enum CollectorRequest {
    metrics(MetricsRequest),
    sysinfo(SystemInfoRequest),
    sysctl(SystemctlRequest),
}

pub async fn metric_collector(mut tx: mpsc::Sender<CollectorRequest>) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    let mut sys = System::new_all();
    tokio::time::sleep(MINIMUM_CPU_UPDATE_INTERVAL).await;
    info!("[agent] Metric collector started, collecting every minute...");
    loop {
        interval.tick().await;
        let now = Instant::now();
        let metrics = lib::system_info::collect_metrics(&mut sys).await;
        let elapsed = now.elapsed();
        info!("[metrics] Collection complete [{:.2?}]", elapsed);
        match tx.try_send(CollectorRequest::metrics(metrics)) {
            Ok(_) => {
                info!(
                    "[metrics collector] queue cap remaining: {} / {}",
                    tx.capacity(),     // remaining space
                    tx.max_capacity()  // total channel capacity
                );
            }
            Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                info!("[metrics] Queue full, dropping sample");
            }
            Err(e) => {
                info!("[metrics] Failed to send metrics: {}", e);
                break;
            }
        }
    }
}

pub async fn sysinfo_collector(mut tx: mpsc::Sender<CollectorRequest>) {
    let mut interval = tokio::time::interval(Duration::from_secs(60 * 10));
    let mut sys = System::new_all();
    info!("[agent] Sysinfo collector started, collecting every 10 minutes...");
    loop {
        let now = Instant::now();
        interval.tick().await;
        let system_info = lib::system_info::collect_system_info(&mut sys).await;
        let elapsed = now.elapsed();
        info!("[sysinfo] Collection complete [{:.2?}]", elapsed);
        match tx.try_send(CollectorRequest::sysinfo(system_info)) {
            Ok(_) => {
                info!(
                    "[sysinfo collector] queue cap remaining: {} / {}",
                    tx.capacity(),     // remaining space
                    tx.max_capacity()  // total channel capacity
                );
            }
            Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                info!("[sysinfo] Queue full, dropping update");
            }
            Err(e) => {
                info!("[sysinfo] Failed to send system info: {}", e);
                break;
            }
        }
    }
}

pub async fn systemctl_collector(mut tx: mpsc::Sender<CollectorRequest>) {
    let mut interval = tokio::time::interval(Duration::from_secs(60 * 5));
    info!("[agent] Systemctl collector started, collecting every 10 minutes...");
    loop {
        interval.tick().await;
        let now = Instant::now();
        let system_info = match timeout(
            Duration::from_secs(15),
            lib::system_info::collect_systemctl_services(),
        )
        .await
        {
            Ok(res) => res,
            Err(_) => {
                info!("[systemctl] Collection timed out, skipping this cycle");
                continue;
            }
        };
        let elapsed = now.elapsed();
        info!("[systemctl] Collection complete [{:.2?}]", elapsed);
        match tx.try_send(CollectorRequest::sysctl(system_info)) {
            Ok(_) => {
                info!(
                    "[systemctl collector] queue cap remaining: {} / {}",
                    tx.capacity(),     // remaining space
                    tx.max_capacity()  // total channel capacity
                );
            }
            Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                info!("[systemctl] Queue full, dropping update");
            }
            Err(e) => {
                info!("[systemctl] Failed to send systemctl info: {}", e);
                break;
            }
        }
    }
}
