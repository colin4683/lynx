mod lib;
mod proto;
use crate::lib::client::{handle_collector_requests, AuthInterceptor, GrpcClient, LynxConfig};
use crate::lib::collectors::CollectorRequest;
use crate::lib::websocket::PeerMap;
use bollard::query_parameters::ListContainersOptions;
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
use tonic::codegen::InterceptedService;
use tonic::metadata::MetadataValue;
use tonic::service::Interceptor;
use tonic::transport::ClientTlsConfig;
use tonic::{Code, Status};

type Tx = UnboundedSender<Message>;
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

    let make_client = |config: &LynxConfig,
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
    let mut grpc_client = GrpcClient::new(client, config, client_tls_config);

    // Start collectors with async mpsc
    let (tx, mut rx) = mpsc::channel::<lib::collectors::CollectorRequest>(1024);

    lib::collectors::start_collectors(tx.clone()).await;

    let mut handles = vec![];

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
                false // Remove a finished handle
            } else {
                true // Keep a running handle
            }
        });

        tokio::select! {
            Some(request) = rx.recv() => {
                if let Err(e) = handle_collector_requests(&mut grpc_client, request).await {
                    error!("[agent] Error handling collector request: {}", e);
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
