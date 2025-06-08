use std::fmt::Debug;
use std::time::Duration;
use serde::Deserialize;
use sysinfo::{Components, ProcessRefreshKind, ProcessesToUpdate};
use crate::proto::monitor::{CpuStats, MemoryStats};
use crate::proto::monitor::system_monitor_client::SystemMonitorClient;
use tonic::metadata::MetadataValue;
use tonic::service::Interceptor;
use tonic::Status;
use toml::Table;


#[derive(Deserialize, Debug)]
pub struct CoreConfig {
    pub server_url: String,
    pub agent_key: String
}

#[derive(Deserialize, Debug)]
pub struct LynxConfig {
    pub core: CoreConfig,
}




struct AuthInterceptor {
    agent_key: String
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

mod proto {
    pub mod monitor {
        tonic::include_proto!("monitor");
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

    loop {
        sys.refresh_all();
        let total_memory = sys.total_memory();
        let used_memory = sys.used_memory();
        let num_cpus = sys.cpus().len();

        // sleep for CPU to update
        tokio::time::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL).await;
        sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_cpu()
        );

        let cpu = sys.global_cpu_usage() / num_cpus as f32;
        let components = Components::new_with_refreshed_list();
        let temperature = components.iter().find(|c| c.label().contains("CPU") || c.label().contains("Tdie") || c.label().contains("Tctl"))
            .map(|c| c.temperature());
        
        let memory_stats = MemoryStats {
            used_bytes: used_memory,
            total_bytes: total_memory,
            free_bytes: total_memory,
        };
        let cpu_stats = CpuStats {
            usage_percent: cpu as f64,
            temperature_c: temperature.unwrap_or(Some(0.0)).unwrap_or(0.0) as f64,
        };
        
        
        // create request
        let request = tonic::Request::new(proto::monitor::MetricsRequest {
            hostname: "127.0.0.1".to_string(),
            cpu: Some(cpu_stats),
            memory: Some(memory_stats),
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
