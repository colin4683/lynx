mod lib;
mod proto;
use crate::lib::websocket::PeerMap;
use dotenv::dotenv;
use env_logger::Env;
use futures_channel::mpsc::UnboundedSender;
use log::{error, info};
use proto::monitor::system_monitor_client::SystemMonitorClient;
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt::Debug;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::protocol::Message;
use tonic::metadata::MetadataValue;
use tonic::service::Interceptor;
use tonic::{Code, Status};

type Tx = UnboundedSender<Message>;

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
    dotenv().ok();
    let env = Env::default()
        .filter("MY_LOG_LEVEL")
        .write_style("MY_LOG_STYLE");
    env_logger::Builder::from_env(env)
        .format_timestamp_secs()
        .init();

    info!("[agent] Starting Lynx Agent...");

    let client_tls_config = lib::client::tls_config().await.map_err(|e| {
        error!("[agent] Failed to load TLS configuration: {}", e);
        e
    })?;

    let config_str = std::fs::read_to_string("config.toml").map_err(|e| {
        error!("[agent] No config.toml found, please create one.");
        e
    })?;

    let config: LynxConfig = toml::from_str(&config_str)?;

    info!("Connecting to lynx-hub at {}", config.core.server_url);

    let mut make_client = |config: &LynxConfig,
                           tls: tonic::transport::ClientTlsConfig|
     -> Result<tonic::transport::Endpoint, Box<dyn std::error::Error>> {
        let endpoint = tonic::transport::Endpoint::from_shared(config.core.server_url.clone())?
            .tls_config(tls)?
            .tcp_keepalive(Some(Duration::from_secs(30)))
            .http2_keep_alive_interval(Duration::from_secs(15))
            .keep_alive_timeout(Duration::from_secs(5))
            .keep_alive_while_idle(true)
            .connect_timeout(Duration::from_secs(10));
        Ok(endpoint)
    };

    // Connect to gRPC server with mTLS
    let endpoint = make_client(&config, client_tls_config.clone())?;
    let channel = endpoint.connect().await?;
    let mut client = SystemMonitorClient::with_interceptor(
        channel,
        AuthInterceptor {
            agent_key: config.core.agent_key.clone(),
        },
    );
    let rpc_timeout = Duration::from_secs(10);

    // Start collectors with async mpsc
    let (tx, mut rx) = mpsc::channel::<lib::collectors::CollectorRequest>(1024);

    let mut handles = vec![];

    // System info collector (cpu model, users, kernal, os,etc.)
    info!("[agent] Starting sysinfo collector...");
    let sysinfo_handle = tokio::spawn(lib::collectors::sysinfo_collector(tx.clone()));
    handles.push(sysinfo_handle);

    // Metric collector (cpu usage, memory usage, disk usage, etc.)
    info!("[agent] Starting metric collector...");
    let metric_handle = tokio::spawn(lib::collectors::metric_collector(tx.clone()));
    handles.push(metric_handle);

    // Systemctl collector (Linux only - get systemd services status)
    //let cache = Arc::new(lib::cache::FastCache::new("sqlite://./cache.db", true).await?);
    #[cfg(target_os = "linux")]
    {
        info!("[agent] Starting systemctl collector...");
        let systemctl_handle = tokio::spawn(lib::collectors::systemctl_collector(tx.clone()));
        handles.push(systemctl_handle);

        /*// Cleanup task for the systemctl cache
         let cache_cleanup_handle = tokio::spawn(lib::cache::start_cleanup_task(
             cache.clone(),
             Duration::from_secs(7 * 60),
         ));
        // handles.push(cache_cleanup_handle);*/
    }
    let state = PeerMap::new(tokio::sync::Mutex::new(HashMap::new()));

    // WebSocket server for real-time updates
    let peers = state.clone();
    let websocket_handle = tokio::spawn(async move {
        let _ = lib::websocket::start_websocket_server(peers).await;
    });
    handles.push(websocket_handle);

    loop {
        // Check if any tasks have finished or panicked
        handles.retain(|handle| {
            if handle.is_finished() {
                info!("[agent] A background task has finished.");
                false // Remove finished handle
            } else {
                true // Keep running handle
            }
        });

        tokio::select! {
            Some(request) = rx.recv() => {
                match request {
                    lib::collectors::CollectorRequest::sysinfo(system_info) => {
                        info!("[agent] Sending system info to hub...");
                        let request = tonic::Request::new(system_info);
                        // Enforce timeout and reconnect on stall
                        match timeout(rpc_timeout, client.get_system_info(request)).await {
                            Ok(Ok(response)) => {
                                let resp = response.into_inner();
                                if resp.status == "200" {
                                    info!("[agent] Successfully sent system info to hub");
                                } else {
                                    info!("[agent] Failed to send system info to hub: {:?}", resp.message);
                                }
                            }
                            Ok(Err(e)) => {
                                error!("[agent] Error sending system info: {}", e);
                                if e.code() == Code::Unavailable || e.code() == Code::DeadlineExceeded {
                                    error!("[agent] Reconnecting gRPC client after error: {:?}", e.code());
                                    let endpoint = make_client(&config, client_tls_config.clone())?;
                                    let channel = endpoint.connect().await?;
                                    client = SystemMonitorClient::with_interceptor(
                                        channel,
                                        AuthInterceptor {
                                            agent_key: config.core.agent_key.clone(),
                                        },
                                    );
                                }
                            }
                            Err(_) => {
                                error!("[agent] Timeout sending system info to hub; reconnecting");
                                let endpoint = make_client(&config, client_tls_config.clone())?;
                                let channel = endpoint.connect().await?;
                                client = SystemMonitorClient::with_interceptor(
                                    channel,
                                    AuthInterceptor {
                                        agent_key: config.core.agent_key.clone(),
                                    },
                                );
                            }
                        }
                    }
                    lib::collectors::CollectorRequest::metrics(metrics) => {
                        info!("[agent] Sending metrics to hub...");
                        let request = tonic::Request::new(metrics);
                        match timeout(rpc_timeout, client.report_metrics(request)).await {
                            Ok(Ok(response)) => {
                                let resp = response.into_inner();
                                if resp.status == "200" {
                                    info!("[agent] Successfully sent metrics to hub");
                                } else {
                                    info!("[agent] Failed to send metrics to hub: {:?}", resp.message);
                                }
                            }
                            Ok(Err(e)) => {
                                error!("[agent] Error sending metrics: {}", e);
                                if e.code() == Code::Unavailable || e.code() == Code::DeadlineExceeded {
                                    error!("[agent] Reconnecting gRPC client after error: {:?}", e.code());
                                    let endpoint = make_client(&config, client_tls_config.clone())?;
                                    let channel = endpoint.connect().await?;
                                    client = SystemMonitorClient::with_interceptor(
                                        channel,
                                        AuthInterceptor {
                                            agent_key: config.core.agent_key.clone(),
                                        },
                                    );
                                }
                            }
                            Err(_) => {
                                error!("[agent] Timeout sending metrics to hub; reconnecting");
                                let endpoint = make_client(&config, client_tls_config.clone())?;
                                let channel = endpoint.connect().await?;
                                client = SystemMonitorClient::with_interceptor(
                                    channel,
                                    AuthInterceptor {
                                        agent_key: config.core.agent_key.clone(),
                                    },
                                );
                            }
                        }
                    }
                    lib::collectors::CollectorRequest::sysctl(systemctl) => {
                        info!("[agent] Sending systemctl services to the hub");
                        let request = tonic::Request::new(systemctl);
                        match timeout(rpc_timeout, client.report_systemctl(request)).await {
                            Ok(Ok(response)) => {
                                let resp = response.into_inner();
                                if resp.status == "200" {
                                    info!("[agent] Successfully sent systemctl services to hub");
                                } else {
                                    info!("[agent] Failed to send systemctl services to hub: {:?}", resp.message);
                                }
                            }
                            Ok(Err(e)) => {
                                error!("[agent] Error sending systemctl services to hub: {}", e);
                                if e.code() == Code::Unavailable || e.code() == Code::DeadlineExceeded {
                                    error!("[agent] Reconnecting gRPC client after error: {:?}", e.code());
                                    let endpoint = make_client(&config, client_tls_config.clone())?;
                                    let channel = endpoint.connect().await?;
                                    client = SystemMonitorClient::with_interceptor(
                                        channel,
                                        AuthInterceptor {
                                            agent_key: config.core.agent_key.clone(),
                                        },
                                    );
                                }
                            }
                            Err(_) => {
                                error!("[agent] Timeout sending systemctl services to hub; reconnecting");
                                let endpoint = make_client(&config, client_tls_config.clone())?;
                                let channel = endpoint.connect().await?;
                                client = SystemMonitorClient::with_interceptor(
                                    channel,
                                    AuthInterceptor {
                                        agent_key: config.core.agent_key.clone(),
                                    },
                                );
                            }
                        }
                    }
                }
            }
            else => {
                // Channel closed
                error!("[agent] All collectors have shut down, exiting main loop.");
                break;
            }
        }
    }
    Ok(())
}
