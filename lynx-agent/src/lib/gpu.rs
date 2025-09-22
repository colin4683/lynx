use crate::proto::monitor::{GpuInfo, GpuMetrics};
use log::{error, info};
use std::process::Stdio;
use tokio::process::Command;
use tokio::sync::Mutex;

lazy_static::lazy_static! {
    static ref NVIDIA_SMI_COMMAND: String = "nvidia-smi".to_string();
    static ref ROCM_SMI_COMMAND: String = "rocm-smi".to_string();
    static ref TEGRASTATS_COMMAND: String = "tegrastats".to_string();

    static ref PREV_GPUS: Mutex<Vec<GpuInfo>> = Mutex::new(Vec::new());
}

pub struct GPUManager {
    nvidia_smi: bool,
    rocm_smi: bool,
    tegrastats: bool,
}

impl GPUManager {
    pub fn new() -> Self {
        let mut manager = Self {
            nvidia_smi: false,
            rocm_smi: false,
            tegrastats: false,
        };
        manager.detect_gpus();
        manager
    }

    fn detect_gpus(&mut self) {
        // execute commands to detect GPUs remove output
        self.nvidia_smi = Command::new(NVIDIA_SMI_COMMAND.as_str())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .is_ok();

        self.rocm_smi = Command::new(ROCM_SMI_COMMAND.as_str())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .is_ok();

        self.tegrastats = Command::new(TEGRASTATS_COMMAND.as_str())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .is_ok();
    }

    pub async fn start_collection(
        &self,
    ) -> Result<
        (Option<Vec<GpuInfo>>, Vec<GpuMetrics>),
        Box<dyn std::error::Error + Send + Sync + 'static>,
    > {
        if self.nvidia_smi {
            match self.collect_nvidia().await {
                Ok((inventory, metrics)) => {
                    let (changed_inventory, gpu_metrics) =
                        self.detect_gpu_changes(inventory, metrics).await;
                    Ok((changed_inventory, gpu_metrics))
                }
                Err(e) => {
                    error!("Failed to collect NVIDIA GPU metrics: {}", e);
                    Err(e)
                }
            }
        } else if self.rocm_smi {
            info!("ROCm GPU detected. Starting ROCm GPU metrics collection.");
            // Start ROCm GPU metrics collection
            Ok((None, Vec::new()))
        } else if self.tegrastats {
            info!("Tegra GPU detected. Starting Tegra GPU metrics collection.");
            // Start Tegra GPU metrics collection
            Ok((None, Vec::new()))
        } else if !self.nvidia_smi && !self.rocm_smi && !self.tegrastats {
            Err("No supported GPUs detected".into())
        } else {
            Ok((None, Vec::new()))
        }
    }

    async fn detect_gpu_changes(
        &self,
        current_inventory: Vec<GpuInfo>,
        metrics: Vec<GpuMetrics>,
    ) -> (Option<Vec<GpuInfo>>, Vec<GpuMetrics>) {
        let mut guard = PREV_GPUS.lock().await;
        let changed = *guard != current_inventory;
        if changed {
            *guard = current_inventory.clone();
            (Some(current_inventory), metrics)
        } else {
            (None, metrics)
        }
    }

    pub async fn collect_nvidia(
        &self,
    ) -> Result<(Vec<GpuInfo>, Vec<GpuMetrics>), Box<dyn std::error::Error + Send + Sync + 'static>>
    {
        if !self.nvidia_smi {
            return Err("NVIDIA SMI not available".into());
        }

        let output = Command::new(NVIDIA_SMI_COMMAND.as_str())
            .arg("--query-gpu=index,name,uuid,pci.bus_id,driver_version,temperature.gpu,memory.used,memory.total,utilization.gpu,power.draw")
            .arg("--format=csv,noheader,nounits")
            .output()
            .await?;
        let output_str = String::from_utf8_lossy(&output.stdout);
        let mut inventory: Vec<GpuInfo> = Vec::new();
        let mut metrics: Vec<GpuMetrics> = Vec::new();
        let now = chrono::Utc::now();
        for line in output_str.lines() {
            // simple CSV split - keeps current behavior; if GPU names contain commas consider a CSV parser
            let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if parts.len() != 10 {
                // Unexpected line, skip
                continue;
            }

            // parse helpers
            let parse_i32 = |s: &str| s.parse::<i32>().ok();
            let parse_i64 = |s: &str| s.parse::<i64>().ok();
            let parse_f32 = |s: &str| s.parse::<f32>().ok();

            let index = parse_i32(parts[0]).unwrap_or(-1);
            let name = Some(parts[1].to_string()).unwrap_or("Unknown".to_string());
            let uuid = match parts[2] {
                "" => None,
                v => Some(v.to_string()),
            };
            let pci_bus = match parts[3] {
                "" => None,
                v => Some(v.to_string()),
            };
            let driver = match parts[4] {
                "" => None,
                v => Some(v.to_string()),
            };
            let temperature: f32 = parse_f32(parts[5]).unwrap_or(0.0);
            let memory_used: i64 = parse_i64(parts[6]).unwrap_or(0);
            // memory.total reported with nounits is in MiB for nvidia-smi
            let memory_total: i64 = parse_i64(parts[7]).unwrap_or(0);
            let utilization: f32 = parse_f32(parts[8]).unwrap_or(0.0);
            let power_draw: f32 = parse_f32(parts[9]).unwrap_or(0.0);

            let gpu_info = GpuInfo {
                gpu_index: index as u32,
                uuid: uuid.clone().unwrap_or_default(),
                name: name.clone(),
                pci_bus: pci_bus.clone().unwrap_or_default(),
                driver: driver.clone().unwrap_or_default(),
                memory_total_mb: memory_total as u64,
            };
            inventory.push(gpu_info);

            let metric = GpuMetrics {
                gpu_index: index as u32,
                temperature: temperature as f64,
                memory_used_mb: memory_used as u64,
                utilization: utilization as f64,
                power: power_draw as f64,
            };
            metrics.push(metric);
        }

        Ok((inventory, metrics))
    }
}
