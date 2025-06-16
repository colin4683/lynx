mod lib;
mod proto;
use std::env;
use crate::proto::monitor::{Component, LoadAverage, MetricsRequest, SystemInfoRequest};
use proto::monitor::system_monitor_client::SystemMonitorClient;
use serde::Deserialize;
use std::fmt::Debug;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;
use env_logger::Env;
use sysinfo::{Components, ProcessRefreshKind, ProcessesToUpdate, System, MINIMUM_CPU_UPDATE_INTERVAL};
use tokio::time::Instant;
use tonic::Status;
use tonic::metadata::MetadataValue;
use tonic::service::Interceptor;
use log::{info, warn, error};
use dotenv::dotenv;

#[derive(Clone)]
pub struct SystemInstance {
    inner: Arc<Mutex<System>>
}

impl SystemInstance {
    pub fn new() -> Self {
        let system = System::new_all();
        Self {
            inner: Arc::new(Mutex::new(system))
        }
    }

    pub fn refresh_and_get<F, R>(&self, refresh: bool, f: F) -> R
    where
        F: FnOnce(&mut System) -> R,
    {
        let mut sys = self.inner.lock().unwrap();
        if refresh {
            sys.refresh_all();
        }
        f(&mut sys)
    }
}

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

enum CollectorRequest {
    metrics(MetricsRequest),
    sysinfo(SystemInfoRequest)
}

async fn metric_collector(tx: mpsc::Sender<CollectorRequest>) {
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
        tx.send(CollectorRequest::metrics(metrics)).unwrap();
    }
}

async fn sysinfo_collector(tx: mpsc::Sender<CollectorRequest>) {
    let mut interval = tokio::time::interval(Duration::from_secs(60 * 10));
    let mut sys = System::new_all();
    info!("[agent] Sysinfo collector started, collecting every 10 minutes...");
    loop {
        let now = Instant::now();
        interval.tick().await;
        let system_info = lib::system_info::collect_system_info(&mut sys).await;
        let elapsed = now.elapsed();
        info!("[sysinfo] Collection complete [{:.2?}]", elapsed);
        tx.send(CollectorRequest::sysinfo(system_info)).unwrap(); // Uncomment if you want to send this data
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // load config
   // env_logger::init();
    dotenv().ok();

    let env = Env::default()
        .filter("MY_LOG_LEVEL")
        .write_style("MY_LOG_STYLE");
    env_logger::Builder::from_env(env)
        .format_timestamp_secs()
        .init();
    info!("[agent] Starting Lynx Agent...");
    
    let config_str = std::fs::read_to_string("config.toml").map_err(|e| {
        error!("[agent] No config.toml found, please create one.");
        e
    })?;
    let config: LynxConfig = toml::from_str(&config_str)?;
    let (tx, rx) = mpsc::channel::<CollectorRequest>();

   info!("Connecting to lynx-hub at {}", config.core.server_url);
   let channel = tonic::transport::Channel::from_shared(config.core.server_url)?
        .connect()
        .await?;

    let mut client = SystemMonitorClient::with_interceptor(channel, AuthInterceptor { agent_key: config.core.agent_key });


    info!("[agent] Starting sysinfo collector...");
    tokio::spawn(sysinfo_collector(tx.clone()));


    info!("[agent] Starting metric collector...");
    tokio::spawn(metric_collector(tx.clone()));

    
    // create a span for tracing
    // wait for messages on the channel, then send to hub 
    loop {
        match rx.recv() {
            Ok(request) => {
                match request {
                    CollectorRequest::sysinfo(system_info) => {
                        info!("[agent] Sending system info to hub...");
                        let request = tonic::Request::new(system_info);
                        match client.get_system_info(request).await {
                            Ok(response) => {
                                let resp = response.into_inner();
                                if resp.status == "200" {
                                    info!("[agent] Successfully sent system info to hub");
                                } else {
                                    info!("[agent] Failed to send system info to hub: {:?}", resp.message)
                                }
                            },
                            Err(e) => {
                                error!("[agent] Error sending system info: {}", e);
                            }
                        }
                    },
                    CollectorRequest::metrics(metrics) => {
                        info!("[agent] Sending metrics to hub...");
                        let request = tonic::Request::new(metrics);
                        match client.report_metrics(request).await {
                            Ok(response) => {
                                let resp = response.into_inner();
                                if resp.status == "200" {
                                    info!("[agent] Successfully sent metrics to hub");
                                } else {
                                    info!("[agent] Failed to send metrics to hub: {:?}", resp.message)
                                }
                            },
                            Err(e) => {
                                error!("[agent] Error sending metrics to hub: {}", e);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!("[agent] Error receiving metrics: {}", e);
            }
        }
    }
}
