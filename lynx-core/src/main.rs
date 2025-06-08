use tonic::{Request, Response, Status};
use crate::proto::monitor::system_monitor_server::{SystemMonitor, SystemMonitorServer};
use crate::proto::monitor::{MetricsRequest, MetricsResponse};
use sqlx::postgres::PgPoolOptions;
use chrono::Utc;
use uuid::uuid;

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
    curl -L https://example.com/agent/lynx-agent -o /usr/local/bind/lynx-agent
    chmod +x /usr/local/bin/lynx-agent
    
    # Create config
    mkdir -p /etc/lynx
    cat > /etc/lynx/config.toml <<EOF
    [core]
    server_url = "grpc://localhost:50051"
    agent_key = "{}"
    EOF
    
    # Start as systemd service
    cat > /etc/systemd/system/lynx-agent.service <<EOF
    [Unit]
    Description=Lynx Agent
    
    [Service]
    ExecStart=/usr/local/bin/lynx-agent
    Restart=always
    
    [Install]
    WantedBy=multi-user.target
    EOF
    
    # Enable service
    systemctl enable lynx-agent
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
            metrics.cpu.unwrap().usage_percent,
            metrics.cpu.unwrap().temperature_c,
            metrics.memory.unwrap().used_bytes as i64,
            metrics.memory.unwrap().total_bytes as i64
        )
            .execute(&self.pool)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;
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
    
    let addr = "[::1]:50051".parse()?;
    let db_pool = setup_db().await?;

    let monitor = MyMonitor {
        pool: db_pool.clone(),
    };

    tonic::transport::Server::builder()
        .add_service(SystemMonitorServer::new(monitor))
        .serve(addr)
        .await?;

    Ok(())
}
