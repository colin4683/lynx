use chrono::Utc;
use log::{error, info};
use serde::{Deserialize, Serialize};
use tonic::{Request, Response, Status};

use crate::cache::Cache;
use crate::proto::monitor::system_monitor_server::SystemMonitor;
use crate::proto::monitor::{
    MetricsRequest, MetricsResponse, SystemInfoRequest, SystemInfoResponse, SystemctlRequest,
    SystemctlResponse,
};

#[derive(Clone)]
pub struct MyMonitor {
    pub pool: sqlx::PgPool,
    pub cache: Cache,
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

        // spawn thread to process notification rules
        let metrics_thread = metrics.clone();
        let pool_clone = self.pool.clone();
        tokio::spawn(async move {
            if let Err(e) =
                crate::notify::process_notification(&metrics_thread, system.id, &pool_clone).await
            {
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
        let components_json = serde_json::to_string(&components).map_err(|e| {
            error!("[hub] Failed to serialize component list: {}", e);
            Status::internal("Serialization error")
        })?;

        let network_stats = metrics.network_stats.unwrap();
        let loads = metrics.load_average.unwrap();

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
            disk_query.execute(&self.pool).await.map_err(|e| {
                error!("[hub] Failed to insert disk: {e:?}");
                Status::internal("Database error")
            })?;
        }

        info!("[hub] Metric log successfully saved");
        // record lightweight log in cache
        let cache = self.cache.clone();
        tokio::spawn(async move {
            cache.record_log("info", "metrics inserted").await;
        });
        Ok(Response::new(MetricsResponse {
            status: "200".to_string(),
            message: "Metrics reported successfully".to_string(),
        }))
    }

    async fn get_system_info(
        &self,
        request: Request<SystemInfoRequest>,
    ) -> Result<Response<SystemInfoResponse>, Status> {
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
        )
        .execute(&self.pool)
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

    async fn report_systemctl(
        &self,
        request: Request<SystemctlRequest>,
    ) -> Result<Response<SystemctlResponse>, Status> {
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
        let request = request.into_inner();
        let services = request.services;
        for service in services {
            // update in-memory cache first for fast reads
            self.cache.upsert_service(service.clone());

            let existing = sqlx::query!(
                r#"SELECT id FROM services WHERE system = $1 AND name = $2"#,
                system.id,
                service.service_name
            )
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                error!("[hub] Failed to query existing service: {e:?}");
                Status::internal("Database error")
            })?;

            if let Some(existing_service) = existing {
                // update existing service
                sqlx::query!(
                    r#"
                    UPDATE services
                    SET description = $1,
                        state = $2,
                        pid = $3,
                        cpu = $4,
                        memory = $5
                    WHERE id = $6
                    "#,
                    service.description,
                    service.state,
                    service.pid as i32,
                    service.cpu,
                    service.memory,
                    existing_service.id
                )
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    error!("[hub] Failed to update service: {e:?}");
                    Status::internal("Database error")
                })?;
                continue;
            } else {
                sqlx::query!(
                    r#"
                    INSERT INTO services (system, name, description, state, pid, cpu, memory)
                    VALUES ($1, $2, $3, $4, $5, $6, $7)
                    "#,
                    system.id,
                    service.service_name,
                    service.description,
                    service.state,
                    service.pid as i32,
                    service.cpu,
                    service.memory
                )
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    error!("[hub] Failed to insert service: {e:?}");
                    Status::internal("Database error")
                })?;
            }
        }

        info!("[hub] Systemctl services updated successfully");
        // log cache size
        let svc_count = self.cache.list_services().len();
        info!("[hub] Cache now tracking {svc_count} services");
        Ok(Response::new(SystemctlResponse {
            status: "200".to_string(),
            message: "Services reported successfully".to_string(),
        }))
    }
}
