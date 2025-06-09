use crate::proto::monitor::system_monitor_server::{SystemMonitor, SystemMonitorServer};
use crate::proto::monitor::{MetricsRequest, MetricsResponse};
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
use tokio::net::TcpListener;
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
        let agent_key = request
            .metadata()
            .get("x-agent-key")
            .ok_or(Status::unauthenticated("Missing key"))?
            .to_str()
            .map_err(|_| Status::invalid_argument("Invalid key"))?;

        let valid = sqlx::query!(
            r#"SELECT cpu, hostname FROM systems WHERE key = $1 AND active = true"#,
            agent_key.clone()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;
        if valid.is_none() {
            return Err(Status::unauthenticated("Invalid or inactive agent key"));
        }

        let system = valid.unwrap();
        let metrics = request.into_inner();
        let components = metrics
            .components
            .iter()
            .map(|c| ComponentJSON {
                label: c.label.clone(),
                temperature: c.temperature,
            })
            .collect::<Vec<_>>();
        let components_json = serde_json::to_string(&components)
            .map_err(|e| Status::internal(format!("Serialization error: {}", e)))?;

        if system.cpu.is_none() {
            sqlx::query!(
                r#"UPDATE systems SET cpu = $1 WHERE hostname = $2"#,
                metrics.cpu_model, metrics.hostname
            ).execute(&self.pool)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;
        }
        
        sqlx::query!(
            r#"
            INSERT INTO metrics (time, system_id, cpu_usage, cpu_temp, memory_used_kb, memory_total_kb, components, uptime)
            VALUES ($1, (SELECT id FROM systems WHERE hostname = $2), $3, $4, $5, $6, $7, $8)
            "#,
            Utc::now(),
            metrics.hostname,
            metrics.cpu_stats.unwrap().usage_percent,
            0.0,
            metrics.memory_stats.unwrap().used_kb as i64,
            metrics.memory_stats.unwrap().total_kb as i64,
            components_json,
            metrics.uptime_seconds as i64
        )
            .execute(&self.pool)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        // store disks
        let disks = metrics
            .disk_stats
            .into_iter()
            .map(|disk| {
                sqlx::query!(
                    r#"
                INSERT INTO disks (time, system, name, space, used, read, write, unit)
                VALUES ($1, (SELECT id FROM systems WHERE hostname = $2), $3, $4, $5, $6, $7, $8)
                "#,
                    Utc::now(),
                    metrics.hostname,
                    disk.name,
                    disk.total_space as i64,
                    disk.used_space as i64,
                    disk.read_bytes as i64,
                    disk.write_bytes as i64,
                    disk.unit
                )
            })
            .collect::<Vec<_>>();

        for disk_query in disks {
            disk_query
                .execute(&self.pool)
                .await
                .map_err(|e| Status::internal(format!("Database error: {}", e)))?;
        }

        Ok(Response::new(MetricsResponse {
            status: "200".to_string(),
            message: "Metrics reported successfully".to_string(),
        }))
    }
}

mod proto;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    // check if DATABASE_URl
    if std::env::var("DATABASE_URL").is_err() {
        eprintln!("DATABASE_URL environment variable is not set.");
        std::process::exit(1);
    }

    let db_pool = setup_db().await?;

    let monitor = MyMonitor {
        pool: db_pool.clone(),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app_state = Arc::new(AppState {
        pool: db_pool.clone(),
    });

    let serve_dir = ServeDir::new(Path::new("../lynx-portal"))
        .not_found_service(ServeFile::new("assets/index.html"))
        .append_index_html_on_directories(true);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener = TcpListener::bind(addr).await?;
    let addr = SocketAddr::from(([0, 0, 0, 0], 50051));
    let server = tonic::transport::Server::builder()
        .add_service(SystemMonitorServer::new(monitor))
        .serve(addr);
    if let Err(e) = server.await {
        eprintln!("Tonic server error: {}", e);
    }
    Ok(())
}
