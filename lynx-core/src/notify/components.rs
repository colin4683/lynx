use super::*;
use crate::proto::monitor::{CpuStats, DiskStats, LoadAverage, MemoryStats, NetworkStats};

// CPU Component Implementation
pub struct CpuComponent {
    stats: Arc<RwLock<CpuStats>>,
}

impl CpuComponent {
    pub fn new(stats: CpuStats) -> Self {
        Self {
            stats: Arc::new(RwLock::new(stats)),
        }
    }
}

#[async_trait]
impl MetricComponent for CpuComponent {
    async fn get_metric(&self, metric_name: &str) -> Result<f64, MetricError> {
        let stats = self.stats.read().await;
        match metric_name {
            "usage" => Ok(stats.usage_percent as f64),
            _ => Err(MetricError::MetricNotFound(format!(
                "CPU metric {} not found",
                metric_name
            ))),
        }
    }

    fn available_metrics(&self) -> Vec<&str> {
        vec!["usage"]
    }
}

// Memory Component Implementation
pub struct MemoryComponent {
    stats: Arc<RwLock<MemoryStats>>,
}

impl MemoryComponent {
    pub fn new(stats: MemoryStats) -> Self {
        Self {
            stats: Arc::new(RwLock::new(stats)),
        }
    }
}

#[async_trait]
impl MetricComponent for MemoryComponent {
    async fn get_metric(&self, metric_name: &str) -> Result<f64, MetricError> {
        let stats = self.stats.read().await;
        match metric_name {
            "used" => Ok(stats.used_kb as f64),
            "total" => Ok(stats.total_kb as f64),
            "usage" => Ok((stats.used_kb as f64 / stats.total_kb as f64) * 100.0),
            _ => Err(MetricError::MetricNotFound(format!(
                "Memory metric {} not found",
                metric_name
            ))),
        }
    }

    fn available_metrics(&self) -> Vec<&str> {
        vec!["used", "total", "usage"]
    }
}

// Disk Component Implementation
pub struct DiskComponent {
    stats: Arc<RwLock<Vec<DiskStats>>>,
}

impl DiskComponent {
    pub fn new(stats: Vec<DiskStats>) -> Self {
        Self {
            stats: Arc::new(RwLock::new(stats)),
        }
    }

    async fn find_main_disk(&self) -> Option<DiskStats> {
        let stats = self.stats.read().await;
        stats.iter().find(|d| d.mount_point == "/").cloned()
    }
}

#[async_trait]
impl MetricComponent for DiskComponent {
    async fn get_metric(&self, metric_name: &str) -> Result<f64, MetricError> {
        let main_disk = self
            .find_main_disk()
            .await
            .ok_or_else(|| MetricError::ComponentNotFound("Main disk (/) not found".to_string()))?;

        match metric_name {
            "used" => Ok(main_disk.used_space as f64),
            "total" => Ok(main_disk.total_space as f64),
            "usage" => Ok((main_disk.used_space as f64 / main_disk.total_space as f64) * 100.0),
            _ => Err(MetricError::MetricNotFound(format!(
                "Disk metric {} not found",
                metric_name
            ))),
        }
    }

    fn available_metrics(&self) -> Vec<&str> {
        vec!["used", "total", "usage"]
    }
}

// Load Average Component Implementation
pub struct LoadComponent {
    stats: Arc<RwLock<LoadAverage>>,
}

impl LoadComponent {
    pub fn new(stats: LoadAverage) -> Self {
        Self {
            stats: Arc::new(RwLock::new(stats)),
        }
    }
}

#[async_trait]
impl MetricComponent for LoadComponent {
    async fn get_metric(&self, metric_name: &str) -> Result<f64, MetricError> {
        let stats = self.stats.read().await;
        match metric_name {
            "one" => Ok(stats.one_minute as f64),
            "five" => Ok(stats.five_minutes as f64),
            "fifteen" => Ok(stats.fifteen_minutes as f64),
            _ => Err(MetricError::MetricNotFound(format!(
                "Load metric {} not found",
                metric_name
            ))),
        }
    }

    fn available_metrics(&self) -> Vec<&str> {
        vec!["one", "five", "fifteen"]
    }
}

// Network Component Implementation
pub struct NetworkComponent {
    stats: Arc<RwLock<NetworkStats>>,
}

impl NetworkComponent {
    pub fn new(stats: NetworkStats) -> Self {
        Self {
            stats: Arc::new(RwLock::new(stats)),
        }
    }
}

#[async_trait]
impl MetricComponent for NetworkComponent {
    async fn get_metric(&self, metric_name: &str) -> Result<f64, MetricError> {
        let stats = self.stats.read().await;
        match metric_name {
            "in" => Ok(stats.r#in as f64),
            "out" => Ok(stats.out as f64),
            _ => Err(MetricError::MetricNotFound(format!(
                "Network metric {} not found",
                metric_name
            ))),
        }
    }

    fn available_metrics(&self) -> Vec<&str> {
        vec!["in", "out"]
    }
}
