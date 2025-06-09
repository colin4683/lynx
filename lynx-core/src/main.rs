use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use crate::proto::monitor::system_monitor_server::{SystemMonitor, SystemMonitorServer};
use crate::proto::monitor::{MetricsRequest, MetricsResponse};
use sqlx::postgres::PgPoolOptions;
use chrono::Utc;
use axum::{Router, routing::get, ServiceExt};
use axum::extract::State;
use axum::response::Html;
use axum_htmx::HxBoosted;
use serde_json::json;
use uuid::uuid;
use serde::Serialize;
use tokio::net::TcpListener;
use tower_http::cors::{CorsLayer, Any};
use tower_http::services::{ServeDir, ServeFile};

async fn setup_db() -> Result<sqlx::PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(std::env::var("DATABASE_URL").unwrap().as_str()).await?;
    Ok(pool)
}


async fn generate_agent_install_script(
    hostname: &str, token: &str, pool: &sqlx::PgPool
) -> Result<String, Box<dyn std::error::Error>> {

    let agent = sqlx::query!(
        r"SELECT * FROM systems WHERE hostname = $1 AND token = $2 AND active = false",
        hostname, token
    ).fetch_optional(pool).await?;

    if agent.is_none() {
        return Err("Invalid hostname or token".into());
    }

    let agent_key = uuid::Uuid::new_v4().to_string();
    
    let script = format!(r##"
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
    "##, agent_key);
    
    // update the agent record with the new key and set it to active
    sqlx::query!(
        r#"UPDATE systems SET active = true, key = $1 WHERE id = $2"#,
        agent_key, agent.unwrap().id
    ).execute(pool).await?;
    Ok(script)
}

#[derive(Clone)]
struct MyMonitor {
    pool: sqlx::PgPool,
}

struct AppState {
    pool: sqlx::PgPool,
}

#[tonic::async_trait]
impl SystemMonitor for MyMonitor {
    async fn report_metrics(
        &self, request: Request<MetricsRequest>
    ) -> Result<Response<MetricsResponse>, Status> {

        let agent_key = request.metadata().get("x-agent-key")
            .ok_or(Status::unauthenticated("Missing key"))?
            .to_str()
            .map_err(|_| Status::invalid_argument("Invalid key"))?;

        let valid = sqlx::query!(
            r#"SELECT * FROM systems WHERE key = $1 AND active = true"#,
            agent_key
        )
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;
        if valid.is_none() {
            return Err(Status::unauthenticated("Invalid or inactive agent key"));
        }

        let metrics = request.into_inner();
        sqlx::query!(
            r#"
            INSERT INTO metrics (time, system_id, cpu_usage, cpu_temp, memory_used_kb, memory_total_kb)
            VALUES ($1, (SELECT id FROM systems WHERE hostname = $2), $3, $4, $5, $6)
            "#,
            Utc::now(),
            metrics.hostname,
            metrics.cpu_stats.unwrap().usage_percent,
            0.0,
            metrics.memory_stats.unwrap().used_kb as i64,
            metrics.memory_stats.unwrap().total_kb as i64
        )
            .execute(&self.pool)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;


        // store disks
        let disks = metrics.disk_stats.into_iter().map(|disk| {
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
        }).collect::<Vec<_>>();

        for disk_query in disks {
            disk_query.execute(&self.pool).await
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

    let serve_dir = ServeDir::new(Path::new("../lynx-portal")).not_found_service(ServeFile::new("assets/index.html"))
        .append_index_html_on_directories(true);


    let app = Router::new()
        .route("/api/metrics", get(get_metrics))
        .route("/systems", get(get_systems))
        .with_state(app_state)
        .fallback_service(serve_dir)
        .layer(cors);


    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener = TcpListener::bind(addr).await?;


    tokio::task::spawn(async move {
        let addr = SocketAddr::from(([0, 0, 0, 0], 50051));
        let server = tonic::transport::Server::builder()
            .add_service(SystemMonitorServer::new(monitor))
            .serve(addr);
        if let Err(e) = server.await {
            eprintln!("Tonic server error: {}", e);
        }
    });
    let _ = axum::serve(listener, app.into_make_service()).await;

    Ok(())
}


#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct Metrics {
    pub time: chrono::DateTime<chrono::Utc>,
    pub system_id: Option<i32>,
    pub cpu_usage: Option<f64>,
    pub cpu_temp: Option<f64>,
    pub memory_used_kb: Option<i64>,
    pub memory_total_kb: Option<i64>,
    pub docker_containers_running: Option<i32>
}

#[derive(Debug, sqlx::FromRow, Serialize)]
struct SystemReturn {
    pub time: chrono::DateTime<chrono::Utc>,
    pub system_id: Option<i32>,
    pub cpu_usage: Option<f64>,
    pub memory_used_kb: Option<i64>,
}
async fn get_systems(State(state): State<Arc<AppState>>,HxBoosted(boosted): HxBoosted) -> Html<String> {


    // get most recent metrics for each system
    let system = sqlx::query_as!(
        SystemReturn,
        r#"
        SELECT time, system_id, cpu_usage, memory_used_kb FROM metrics
        ORDER BY time DESC LIMIT 1
        "#
    )
    .fetch_all(&state.pool)
    .await
    .unwrap();

    println!("{:?}", system);

    let rows: String = sqlx::query!("SELECT * FROM systems")
        .fetch_all(&state.pool)
        .await
        .unwrap()
        .iter()
        .map(|s| {
            let system_metrics = system.iter().find(|m| m.system_id == Some(s.id));
            println!("System metrics: {:?}", system_metrics);
            let cpu_usage = system_metrics.map_or("N/A".to_string(), |m| m.cpu_usage.map_or("N/A2".to_string(), |u| format!("{:.2}%", u * 100.0)));
            let memory_used = system_metrics.map_or("N/A".to_string(), |m| m.memory_used_kb.map_or("N/A2".to_string(), |u| format!("{} GB", u / 1024 / 1024 / 1024)));
            format!(
                "<tr>
                    <td>{}</td>
                    <td>{}</td>
                    <td>{}</td>
                    <td>{}</td>
                    <td><a href=\"/systems/{}\" class=\"btn btn-primary\">View</a></td>
                </tr>",
                s.hostname, s.address, cpu_usage, memory_used, s.id
            )
        })
        .collect::<String>();

    if boosted {
        Html(format!(include_str!("../../lynx-portal/index.html"), rows))
    } else {
        Html(format!(include_str!("../../lynx-portal/systems.html"), rows))
    }
}

async fn get_metrics(
    State(state): State<Arc<AppState>>
) -> String {
    let metrics: Vec<Metrics> = sqlx::query_as!(Metrics, "SELECT * FROM metrics ORDER BY time DESC LIMIT 10")
        .fetch_all(&state.pool)
        .await
        .unwrap();
    serde_json::to_string(&metrics).unwrap()
}