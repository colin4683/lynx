mod lib;
mod proto;
use dotenv::dotenv;
use env_logger::{Env};
use futures_channel::mpsc::{UnboundedSender};
use log::{error, info};
use proto::monitor::system_monitor_client::SystemMonitorClient;
use serde::{Deserialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::protocol::Message;
use tonic::Status;
use tonic::metadata::MetadataValue;
use tonic::service::Interceptor;
use crate::lib::websocket::PeerMap;

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

    // connect to grpc
    let channel = tonic::transport::Channel::from_shared(config.core.server_url)?
        .tls_config(client_tls_config)?
        .connect()
        .await?;

    let mut client = SystemMonitorClient::with_interceptor(
        channel,
        AuthInterceptor {
            agent_key: config.core.agent_key,
        },
    );

    // start collectors with async mpsc
    let (tx, mut rx) = mpsc::channel::<lib::collectors::CollectorRequest>(32);

    info!("[agent] Starting sysinfo collector...");
    let sysinfo_handle = tokio::spawn(lib::collectors::sysinfo_collector(tx.clone()));

    info!("[agent] Starting metric collector...");
    let metric_handle = tokio::spawn(lib::collectors::metric_collector(tx.clone()));

    let state = PeerMap::new(tokio::sync::Mutex::new(HashMap::new()));

    // start websocket server
    let peers = state.clone();
    let websocket_handle = tokio::spawn(async move {
        let _ = lib::websocket::start_websocket_server(peers).await;
    });

    // Store handles in a vector for easy polling
    let mut handles = vec![sysinfo_handle, metric_handle, websocket_handle];

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
                        match client.get_system_info(request).await {
                            Ok(response) => {
                                let resp = response.into_inner();
                                if resp.status == "200" {
                                    info!("[agent] Successfully sent system info to hub");
                                } else {
                                    info!(
                                        "[agent] Failed to send system info to hub: {:?}",
                                        resp.message
                                    )
                                }
                            }
                            Err(e) => {
                                error!("[agent] Error sending system info: {}", e);
                            }
                        }
                    }
                    lib::collectors::CollectorRequest::metrics(metrics) => {
                        info!("[agent] Sending metrics to hub...");
                        let request = tonic::Request::new(metrics);
                        match client.report_metrics(request).await {
                            Ok(response) => {
                                let resp = response.into_inner();
                                if resp.status == "200" {
                                    info!("[agent] Successfully sent metrics to hub");
                                } else {
                                    info!(
                                        "[agent] Failed to send metrics to hub: {:?}",
                                        resp.message
                                    )
                                }
                            }
                            Err(e) => {
                                error!("[agent] Error sending metrics to hub: {}", e);
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
