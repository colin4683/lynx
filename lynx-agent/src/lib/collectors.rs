use crate::lib;
use crate::lib::cache::FastCache;
use crate::proto::monitor::{
    GpuMetricsRequest, GpuRequest, GpuResponse, MetricsRequest, SystemInfoRequest, SystemctlRequest,
};
use async_trait::async_trait;
use log::{error, info};
use std::sync::Arc;
use std::time::Duration;
use sysinfo::{System, MINIMUM_CPU_UPDATE_INTERVAL};
use tokio::sync::mpsc;
use tokio::time::{timeout, Instant};

#[derive(Debug, thiserror::Error)]
pub enum CollectorError {
    #[error("Failed to collect metrics: {0}")]
    MetricsCollectionError(String),
    #[error("Failed to collect system info: {0}")]
    SystemInfoCollectionError(String),
    #[error("Failed to collect systemctl info: {0}")]
    SystemctlCollectionError(String),

    #[error("Channel send error: {0}")]
    Channel(#[from] tokio::sync::mpsc::error::TrySendError<CollectorRequest>),
}
#[derive(Debug)]
pub enum CollectorRequest {
    Metrics(MetricsRequest),
    GpuInfo(GpuRequest),
    GpuMetrics(GpuMetricsRequest),
    SystemInfo(SystemInfoRequest),
    Systemctl(SystemctlRequest),
}

#[async_trait]
pub trait Collector: Send + Sync {
    fn name(&self) -> &'static str;

    fn interval(&self) -> u64;

    async fn collect(
        &self,
        tx: mpsc::Sender<CollectorRequest>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>;
}

pub struct CollectorManager {
    collectors: Vec<Arc<dyn Collector>>,
}

impl CollectorManager {
    pub fn new() -> Self {
        Self {
            collectors: Vec::new(),
        }
    }

    pub fn register<C: Collector + 'static>(&mut self, collector: C) {
        self.collectors.push(Arc::new(collector));
    }

    pub async fn start_all(&self, tx: mpsc::Sender<CollectorRequest>) {
        for collector in &self.collectors {
            let tx = tx.clone();
            let collector = Arc::clone(collector);

            tokio::spawn(async move {
                info!("[collector] Starting {} collector", collector.name());
                let mut interval = tokio::time::interval(Duration::from_secs(collector.interval()));

                loop {
                    interval.tick().await;
                    let start = Instant::now();
                    match collector.collect(tx.clone()).await {
                        Ok(_) => {
                            let elapsed = start.elapsed();
                            info!(
                                "[{}][{}s] collection completed",
                                collector.name(),
                                elapsed.as_secs_f32().round()
                            );
                        }
                        Err(e) => {
                            error!("[collector] {} collection failed: {}", collector.name(), e);
                        }
                    }
                }
            });
        }
    }
}

pub struct MetricsCollector;

#[async_trait]
impl Collector for MetricsCollector {
    fn name(&self) -> &'static str {
        "MetricsCollector"
    }

    fn interval(&self) -> u64 {
        60
    }

    async fn collect(
        &self,
        tx: mpsc::Sender<CollectorRequest>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        // collect system metrics and send
        let mut sys = System::new_all();
        tokio::time::sleep(MINIMUM_CPU_UPDATE_INTERVAL).await;
        let metrics = lib::system_info::collect_metrics(&mut sys).await;
        tx.send(CollectorRequest::Metrics(metrics))
            .await
            .map_err(|e| CollectorError::Channel(e.into()))?;

        // collect GPU inventory + metrics and send if present
        let gpu_manager = lib::gpu::GPUManager::new();
        match gpu_manager.start_collection().await {
            Ok((gpu_info_opt, gpu_metrics)) => {
                if let Some(info) = gpu_info_opt {
                    tx.send(CollectorRequest::GpuInfo(GpuRequest { gpus: info }))
                        .await
                        .map_err(|e| CollectorError::Channel(e.into()))
                        .unwrap_or_else(|e| error!("[collector] failed to send GpuInfo: {}", e));
                }

                if !gpu_metrics.is_empty() {
                    tx.send(CollectorRequest::GpuMetrics(GpuMetricsRequest {
                        gpu_metrics,
                    }))
                    .await
                    .map_err(|e| CollectorError::Channel(e.into()))
                    .unwrap_or_else(|e| error!("[collector] failed to send GpuMetrics: {}", e));
                }
                Ok(())
            }
            Err(e) => {
                error!("Failed to collect GPU metrics: {}", e);
                Ok(())
            }
        }
    }
}

pub struct SystemInfoCollector;

#[async_trait]
impl Collector for SystemInfoCollector {
    fn name(&self) -> &'static str {
        "SystemInfoCollector"
    }

    fn interval(&self) -> u64 {
        600
    }

    async fn collect(
        &self,
        tx: mpsc::Sender<CollectorRequest>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        let mut sys = System::new_all();
        let system_info = lib::system_info::collect_system_info(&mut sys).await;
        let request = CollectorRequest::SystemInfo(system_info);
        tx.send(request)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync + 'static>)
    }
}
#[cfg(target_os = "linux")]
pub struct SystemctlCollector;

#[cfg(target_os = "linux")]
#[async_trait]
impl Collector for SystemctlCollector {
    fn name(&self) -> &'static str {
        "SystemctlCollector"
    }

    fn interval(&self) -> u64 {
        300
    }

    async fn collect(
        &self,
        tx: mpsc::Sender<CollectorRequest>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        let systemctl_info = lib::system_info::collect_systemctl_services().await;
        let request = CollectorRequest::Systemctl(systemctl_info);
        tx.send(request)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync + 'static>)
    }
}

pub async fn start_collectors(tx: mpsc::Sender<CollectorRequest>) {
    let mut manager = CollectorManager::new();

    manager.register(MetricsCollector);
    manager.register(SystemInfoCollector);

    #[cfg(target_os = "linux")]
    manager.register(SystemctlCollector);

    manager.start_all(tx).await;
}
