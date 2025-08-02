use std::fs;
use crate::proto::monitor::system_monitor_server::{SystemMonitor, SystemMonitorServer};
use crate::proto::monitor::{MetricsRequest, MetricsResponse, SystemInfoRequest, SystemInfoResponse};
use axum::extract::State;
use axum::response::Html;
use axum::{Router, ServiceExt, routing::get};
use axum_htmx::HxBoosted;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use env_logger::Env;
use log::{error, info};
use tokio::net::TcpListener;
use tonic::transport::server::{ServerTlsConfig};
use tonic::{Request, Response, Status};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use uuid::uuid;

async fn setup_db() -> Result<sqlx::PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(std::env::var("DATABASE_URL").unwrap().as_str())
        .await?;
    Ok(pool)
}

async fn generate_agent_install_script(
    hostname: &str,
    token: &str,
    pool: &sqlx::PgPool,
) -> Result<String, Box<dyn std::error::Error>> {
    let agent = sqlx::query!(
        r"SELECT * FROM systems WHERE hostname = $1 AND token = $2 AND active = false",
        hostname,
        token
    )
    .fetch_optional(pool)
    .await?;

    if agent.is_none() {
        return Err("Invalid hostname or token".into());
    }

    let agent_key = uuid::Uuid::new_v4().to_string();

    let script = format!(
        r##"
    #!/bin/bash
    # Auto-generated install script for Lynx Agent
    
    # Download binary
    curl -L https://example.com/agent/lynx-agent -o /usr/local/bind/lynx-view-agent
    chmod +x /usr/local/bin/lynx-view-agent
    
    # Create config
    mkdir -p /etc/lynx-view
    cat > /etc/lynx-view/config.toml <<EOF
    [core]
    server_url = "grpc://localhost:50051"
    agent_key = "{}"
    EOF
    
    # Start as systemd service
    cat > /etc/systemd/system/lynx-view-agent.service <<EOF
    [Unit]
    Description=Lynx Agent
    
    [Service]
    ExecStart=/usr/local/bin/lynx-view-agent
    Restart=always
    
    [Install]
    WantedBy=multi-user.target
    EOF
    
    # Enable service
    systemctl enable lynx-view-agent
    "##,
        agent_key
    );

    // update the agent record with the new key and set it to active
    sqlx::query!(
        r#"UPDATE systems SET active = true, key = $1 WHERE id = $2"#,
        agent_key,
        agent.unwrap().id
    )
    .execute(pool)
    .await?;
    Ok(script)
}

#[derive(Clone)]
struct MyMonitor {
    pool: sqlx::PgPool,
}

struct AppState {
    pool: sqlx::PgPool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ComponentJSON {
    label: String,
    temperature: f32,
}

#[tonic::async_trait]
impl SystemMonitor for MyMonitor {
    async fn report_metrics(
        &self,
        request: Request<MetricsRequest>,
    ) -> Result<Response<MetricsResponse>, Status> {
        info!("[hub] New metrics request");
        let agent_key = request
            .metadata()
            .get("x-agent-key")
            .ok_or(Status::unauthenticated("Missing key"))?
            .to_str()
            .map_err(|e| {
                error!("[hub] Authorization failed for agent: {e:?}");
                Status::invalid_argument("Invalid key")
            })?;

        let valid = sqlx::query!(
            r#"SELECT id, cpu, hostname FROM systems WHERE key = $1 AND active = true"#,
            agent_key
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            error!("[hub] Failed to find agent for key: {:?}", agent_key);
            Status::internal(format!("Database error: {}", e))
        })?;
        if valid.is_none() {
            error!("[hub] Invalid system for agent key: {:?}", agent_key);
            return Err(Status::unauthenticated("Invalid or inactive agent key"));
        }
        

        let system = valid.unwrap();
        let metrics = request.into_inner();

        // spawn thread to process notification rukes
        let metrics_thread = metrics.clone();
        let pool_clone = self.pool.clone();
        tokio::spawn(async move {
            if let Err(e) = lib::notify::process_notification(&metrics_thread, system.id, &pool_clone).await {
                error!("[hub] Failed to process notification rules: {}", e);
            }
        });
        
        let components = metrics
            .components
            .iter()
            .map(|c| ComponentJSON {
                label: c.label.clone(),
                temperature: c.temperature,
            })
            .collect::<Vec<_>>();
        let components_json = serde_json::to_string(&components)
            .map_err(|e| {
                error!("[hub] Failed to serialize component list: {}", e);
                Status::internal("Serialization error")
            })?;
        
        let network_stats = metrics.network_stats.unwrap();
        let loads = metrics.load_average.unwrap(); // hehe... load
        
        sqlx::query!(
            r#"
            INSERT INTO metrics (time, system_id, cpu_usage, memory_used_kb, memory_total_kb, components, net_in, net_out, load_one, load_five, load_fifteen)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
            Utc::now(),
            system.id,
            metrics.cpu_stats.unwrap().usage_percent,
            metrics.memory_stats.unwrap().used_kb as i64,
            metrics.memory_stats.unwrap().total_kb as i64,
            components_json,
            network_stats.r#in as i64,
            network_stats.r#out as i64,
            loads.one_minute,
            loads.five_minutes,
            loads.fifteen_minutes
        )
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!("[hub] Failed to insert metric log: {e:?}");
                Status::internal("Database error")
            })?;

        // store disks
        let disks = metrics
            .disk_stats
            .into_iter()
            .map(|disk| {
                sqlx::query!(
                    r#"
                INSERT INTO disks (time, system, name, space, used, read, write, unit, mount_point)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#,
                    Utc::now(),
                    system.id,
                    disk.name,
                    disk.total_space as i64,
                    disk.used_space as i64,
                    disk.read_bytes as f64,
                    disk.write_bytes as f64,
                    disk.unit,
                    disk.mount_point
                )
            })
            .collect::<Vec<_>>();

        for disk_query in disks {
            disk_query
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    error!("[hub] Failed to insert disk: {e:?}");
                    Status::internal("Database error")
                })?;
        }

        info!("[hub] Metric log successfully saved");
        
        Ok(Response::new(MetricsResponse {
            status: "200".to_string(),
            message: "Metrics reported successfully".to_string(),
        }))
    }

    async fn get_system_info(&self, request: Request<SystemInfoRequest>) -> Result<Response<SystemInfoResponse>, Status> {
        info!("[hub] New system info request");
        let agent_key = request
            .metadata()
            .get("x-agent-key")
            .ok_or(Status::unauthenticated("Missing key"))?
            .to_str()
            .map_err(|e| {
                error!("[hub] Authorization failed for agent: {e:?}");
                Status::invalid_argument("Invalid key")
            })?;

        let valid = sqlx::query!(
            r#"SELECT id, cpu, hostname FROM systems WHERE key = $1 AND active = true"#,
            agent_key
        )
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                error!("[hub] Failed to find active agent for key: {:?}", agent_key);
                Status::internal(format!("Database error: {}", e))
            })?;
        
        if valid.is_none() {
            error!("[hub] No system info found for agent key: {:?}", agent_key);
            return Err(Status::unauthenticated("Invalid or inactive agent key"));
        }

        let system = valid.unwrap();
        let system_request = request.into_inner();
        
        sqlx::query!(
            r#"
            UPDATE systems 
            SET hostname = $1,
                os = $2,
                uptime = $3,
                kernal = $4,
                cpu = $5,
                cpu_count = $6
            WHERE id = $7
            "#,
            system_request.hostname,
            system_request.os,
            system_request.uptime_seconds as i32,
            system_request.kernel_version,
            system_request.cpu_model,
            system_request.cpu_count as i32,
            system.id
        ).execute(&self.pool)
            .await
            .map_err(|e| {
                error!("[hub] Failed to update system info: {:?}", e);
                Status::internal(format!("Database error: {}", e))
            })?;
        
        info!("[hub] System info updated successfully");
        
        
        Ok(Response::new(SystemInfoResponse {
            status: "200".to_string(),
            message: "Metrics reported successfully".to_string(),
        }))
    }
}

mod proto;
mod lib;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let current_dir = std::env::current_dir()?;
    // certs are in current_dir/certs
    let certs_dir = current_dir.join("certs");
    if !certs_dir.exists() {
        info!("[hub] Certificates directory not found: {:?}", certs_dir);
        std::process::exit(1);
    }
    let server_cert_path = certs_dir.join("server.crt");
    let server_key_path = certs_dir.join("server.key");
    if !server_cert_path.exists() || !server_key_path.exists() {
        info!("[hub] Server certificate or key not found in {:?}", certs_dir);
        std::process::exit(1);
    }

    let server_cert = fs::read_to_string(server_cert_path)?;
    let server_key = fs::read_to_string(server_key_path)?;
    if server_cert.is_empty() || server_key.is_empty() {
        info!("[hub] Server certificate or key is empty");
        std::process::exit(1);
    }
    // CA certificate
    let ca_cert_path = certs_dir.join("ca.crt");
    if !ca_cert_path.exists() {
        info!("[hub] CA certificate not found in {:?}", certs_dir);
        std::process::exit(1);
    }
    if !Path::new("certs/ca.crt").exists() {
        info!("[hub] CA certificate not found in certs directory");
        std::process::exit(1);
    }
    let ca_cert = fs::read_to_string("certs/ca.crt")?;
    let server_tls_config = ServerTlsConfig::new()
        .identity(tonic::transport::Identity::from_pem(server_cert, server_key))
        .client_ca_root(tonic::transport::Certificate::from_pem(ca_cert))
        .client_auth_optional(false);


    let env = Env::default()
        .filter("MY_LOG_LEVEL")
        .write_style("MY_LOG_STYLE");
    env_logger::Builder::from_env(env)
        .format_timestamp_secs()
        .init();
    info!("[hub] Starting Lynx Hub...");

    // check if DATABASE_URl
    if std::env::var("DATABASE_URL").is_err() {
        error!("[hub] DATABASE_URL environment variable is not set.");
        std::process::exit(1);
    }

    let db_pool = setup_db().await?;
    
    info!("[hub] connected to database");

    let monitor = MyMonitor {
        pool: db_pool.clone(),
    };
    let addr = SocketAddr::from(([0, 0, 0, 0], 50051));
    let server = tonic::transport::Server::builder()
        .tls_config(server_tls_config)?
        .add_service(SystemMonitorServer::new(monitor))
        .serve(addr);
    
    info!("[hub] started on https://{}", addr);
    
    if let Err(e) = server.await {
        error!("Tonic server error: {}", e);
    }
    Ok(())
}
