use tokio::sync::mpsc;
use std::time::Duration;
use log::info;
use sysinfo::{System, MINIMUM_CPU_UPDATE_INTERVAL};
use tokio::time::Instant;
use crate::lib;
use crate::proto::monitor::{MetricsRequest, SystemInfoRequest};

pub enum CollectorRequest {
    metrics(MetricsRequest),
    sysinfo(SystemInfoRequest)
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
        if let Err(e) = tx.send(CollectorRequest::metrics(metrics)).await {
            info!("[metrics] Failed to send metrics: {}", e);
            break;
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
        if let Err(e) = tx.send(CollectorRequest::sysinfo(system_info)).await {
            info!("[sysinfo] Failed to send system info: {}", e);
            break;
        }
    }
}