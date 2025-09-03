mod cache;
mod config;
mod db;
mod notify;
mod proto;
mod services;
mod tls; // added cache module

use crate::cache::Cache;
use crate::proto::monitor::system_monitor_server::SystemMonitorServer;
use crate::services::monitor::MyMonitor;
use log::{error, info};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::time::interval;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load env and initialize logging
    config::load_env();
    config::init_logging();
    info!("[hub] Starting Lynx Hub...");

    // Validate database URL
    let database_url = match config::database_url() {
        Ok(url) => url,
        Err(_) => {
            error!("[hub] DATABASE_URL environment variable is not set.");
            std::process::exit(1);
        }
    };

    // Setup DB
    let db_pool = match db::setup_db(&database_url).await {
        Ok(pool) => {
            info!("[hub] Connected to database");
            pool
        }
        Err(e) => {
            error!("[hub] Failed to connect to database: {e}");
            std::process::exit(1);
        }
    };

    // TLS configuration
    let current_dir = std::env::current_dir()?;
    let certs_dir = current_dir.join("certs");
    let server_tls_config = match crate::tls::build_tls_config(&certs_dir) {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("[hub] TLS configuration failed: {e}");
            std::process::exit(1);
        }
    };

    // Build and run gRPC server
    let cache = Cache::new(10_000, 1_000);
    let snapshot_path = current_dir.join("cache.snapshot");
    if let Err(e) = cache.load_from_file(&snapshot_path).await {
        error!("[hub] Failed to load cache snapshot: {e}");
    } else {
        info!("[hub] Cache snapshot loaded");
    }
    // periodic snapshot task
    {
        let cache_clone = cache.clone();
        let snapshot_path_clone = snapshot_path.clone();
        tokio::spawn(async move {
            let mut tick = interval(Duration::from_secs(60));
            loop {
                tick.tick().await;
                if let Err(e) = cache_clone.snapshot_to_file(&snapshot_path_clone).await {
                    log::warn!("[hub] Cache snapshot failed: {e}");
                }
            }
        });
    }
    let monitor = MyMonitor {
        pool: db_pool.clone(),
        cache: cache.clone(),
    };
    let addr = SocketAddr::from(([0, 0, 0, 0], 50051));
    info!("[hub] gRPC server starting on https://{addr}");

    if let Err(e) = tonic::transport::Server::builder()
        .tls_config(server_tls_config)?
        .add_service(SystemMonitorServer::new(monitor))
        .serve(addr)
        .await
    {
        error!("[hub] RPC server error: {e}");
    }

    Ok(())
}
