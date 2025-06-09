mod proto;
use proto::monitor::system_monitor_client::SystemMonitorClient;
use proto::monitor::{CpuStats, MemoryStats, };
use serde::Deserialize;
use std::fmt::Debug;
use std::time::Duration;
use sysinfo::{Components, ProcessRefreshKind, ProcessesToUpdate};
use toml::Table;
use tonic::Status;
use tonic::metadata::MetadataValue;
use tonic::service::Interceptor;
use crate::proto::monitor::Component;

#[derive(Deserialize, Debug)]
pub struct CoreConfig {
    pub server_url: String,
    pub agent_key: String,
}

#[derive(Deserialize, Debug)]
pub struct LynxConfig {
    pub core: CoreConfig,
}

struct AuthInterceptor {
    agent_key: String,
}

impl Interceptor for AuthInterceptor {
    fn call(&mut self, mut request: tonic::Request<()>) -> Result<tonic::Request<()>, Status> {
        request.metadata_mut().insert(
            "x-agent-key",
            MetadataValue::try_from(&self.agent_key).unwrap(),
        );
        Ok(request)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // load config
    let config_str = std::fs::read_to_string("config.toml")?;
    let config: LynxConfig = toml::from_str(&config_str)?;

    let channel = tonic::transport::Channel::from_shared(config.core.server_url)?
    .connect()
    .await?;

    let mut client = SystemMonitorClient::with_interceptor(channel, AuthInterceptor { agent_key: config.core.agent_key });

    let mut sys = sysinfo::System::new_all();
    let host = sysinfo::System::host_name()
        .unwrap_or_else(|| "unknown".to_string());
    let os_version = sysinfo::System::os_version()
        .unwrap_or_else(|| "0.0.0".to_string());
    let os = sysinfo::System::distribution_id();
    let kernal = sysinfo::System::kernel_version()
        .unwrap_or_else(|| "0.0.0".to_string());
    
    

    loop {
        sys.refresh_all();
        // - memory / num_cpus
        let total_memory = sys.total_memory();
        let used_memory = sys.used_memory();
        
        let uptime = sysinfo::System::uptime();

        let memory_stats = MemoryStats {
            used_kb: used_memory,
            total_kb: total_memory,
            free_kb: total_memory,
        };
        
        let hostname = sysinfo::System::host_name().unwrap_or("unknown".to_string());

        let num_cpus = sys.cpus().len();
        // sleep for CPU to update
        tokio::time::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL).await;
        sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_cpu(),
        );

        /*
        System Info:
        * Hostname [x]
        * OS [x]
        * Uptime
        * Kernel version
        * CPU model / count
        * Memory total / used

        Metrics:
        * CPU usage [x]
        * Docker usage
        * Memory usage [x]
        * Docker memory usage
        * Disk usage [x]
        * Disk I/O [x]
        * Bandwidth usage
        * Docker bandwidth usage
        * Temperature (dump sysinfo components) [x]
        * GPU[n] usage (if available)
        * GPU[n] temperature (if available)
         */

        // - cpu / components
        let cpu = sys.global_cpu_usage() / num_cpus as f32;
        
        let components = Components::new_with_refreshed_list();
        let components_dump = components.iter()
            .map(|c| {
                let temp = c.temperature().unwrap_or(0.0);
                Component {
                    label: c.label().to_string(),
                    temperature: temp,
                }
            })
            .collect::<Vec<Component>>();
        let cpu_stats = CpuStats {
           usage_percent: cpu as f64,
        };
        
        let cpuName = sys.cpus().get(0)
            .map(|cpu| cpu.brand().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let sys_disks = sysinfo::Disks::new_with_refreshed_list();
        let mut sys_disks = sys_disks.into_iter()
            .filter(|d| (((d.mount_point().to_str().unwrap_or("") == "/"))
                || (d.mount_point().to_str().unwrap_or("") == "/mnt")))
            .collect::<Vec<_>>();
        
        
        let disks = sys_disks.into_iter().map(|d| {
            let total_space = d.total_space();
            let available_space = d.available_space();
            let write_bytes = d.usage().total_written_bytes;
            let read_bytes = d.usage().total_read_bytes;
            (d.name().display().to_string(), total_space, available_space, write_bytes, read_bytes)
        }).collect::<Vec<_>>()
        .into_iter()
        .map(|(name, total_space, available_space, write_bytes, read_bytes)| {
            proto::monitor::DiskStats {
                name: name,
                used_space: ((total_space - available_space) / 1024 / 1024 / 1024) as i32,
                total_space: (total_space / 1024 / 1024 / 1024) as i32,
                read_bytes: read_bytes as i32,
                write_bytes: write_bytes as i32,
                unit: "gb".to_string(),
            }
        }).collect::<Vec<_>>();
        
        // create request
        let request = tonic::Request::new(proto::monitor::MetricsRequest {
            hostname,
            os: os.to_string(),
            cpu_count: num_cpus as u32,
            cpu_model: cpuName,
            kernel_version: "".to_string(),
            uptime_seconds: uptime,
            memory_stats: Some(memory_stats),
            cpu_stats: Some(cpu_stats),
            disk_stats: disks,
            components: components_dump
        });

        // send request
        match client.report_metrics(request).await {
            Ok(response) => {
                println!("Response: {:?}", response.into_inner());
            },
            Err(e) => {
                eprintln!("Error sending metrics: {:?}", e);
            }
        }
        
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}

