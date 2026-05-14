use crate::cache::Cache;
use crate::proto::monitor::system_monitor_server::SystemMonitor;
use crate::proto::monitor::{
    ContainerInfo, ContainerMetrics, ContainerMetricsRequest, ContainerRequest, ContainerResponse,
    GpuInfo, GpuMetrics, GpuMetricsRequest, GpuRequest, GpuResponse, MetricsRequest,
    MetricsResponse, Response as ProtoResponse, SystemInfoRequest, SystemInfoResponse,
    SystemctlRequest, SystemctlResponse,
};
use crate::services::ingest::{ContainerIngestItem, DiskEntry, IngestItem, MetricIngestItem};
use chrono::Utc;
use log::{error, info};
use serde::{Deserialize, Serialize};
use sqlx::QueryBuilder;
use tokio::sync::mpsc::Sender;
use tonic::codegen::tokio_stream::StreamExt;
use tonic::metadata::MetadataMap;
use tonic::{Request, Response, Status, Streaming};

#[derive(Clone)]
pub struct MyMonitor {
    pub pool: sqlx::PgPool,
    pub cache: Cache,
    pub metric_tx: Sender<IngestItem>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ComponentJSON {
    label: String,
    temperature: f32,
}

impl MyMonitor {
    async fn get_system_id_from_md(&self, md: &MetadataMap) -> Result<i32, Status> {
        let agent_key = md
            .get("x-agent-key")
            .ok_or(Status::unauthenticated("Missing key"))?
            .to_str()
            .map_err(|_| Status::invalid_argument("Invalid key"))?;

        if let Some(id) = self.cache.get_system_id(agent_key) {
            return Ok(id);
        }

        let rec = sqlx::query!(
            r#"SELECT id FROM systems WHERE key = $1 AND active = true"#,
            agent_key
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            error!("[hub] DB system lookup error: {e}");
            Status::internal("Database error")
        })?
        .ok_or(Status::unauthenticated("Invalid or inactive agent key"))?;

        self.cache.put_system_id(agent_key.to_string(), rec.id);
        Ok(rec.id)
    }

    async fn handle_metrics_message(
        &self,
        system_id: i32,
        metrics: crate::proto::monitor::MetricsRequest,
    ) -> Result<(), Status> {
        let cpu = metrics
            .cpu_stats
            .ok_or(Status::invalid_argument("missing cpu_stats"))?;
        let mem = metrics
            .memory_stats
            .ok_or(Status::invalid_argument("missing memory_stats"))?;
        let net = metrics
            .network_stats
            .ok_or(Status::invalid_argument("missing network_stats"))?;
        let load = metrics
            .load_average
            .ok_or(Status::invalid_argument("missing load_average"))?;

        let components_json = serde_json::to_string(
            &metrics
                .components
                .iter()
                .map(|c| ComponentJSON {
                    label: c.label.clone(),
                    temperature: c.temperature,
                })
                .collect::<Vec<_>>(),
        )
        .unwrap_or("[]".to_string());

        let now = chrono::Utc::now();
        let disks = metrics
            .disk_stats
            .iter()
            .map(|d| DiskEntry {
                name: d.name.clone(),
                total_space: d.total_space as i64,
                used_space: d.used_space as i64,
                read_bytes: d.read_bytes,
                write_bytes: d.write_bytes,
                unit: d.unit.clone(),
                mount_point: d.mount_point.clone(),
            })
            .collect::<Vec<_>>();

        let item = IngestItem::Metric(MetricIngestItem {
            system_id,
            time: now,
            cpu_usage: cpu.usage_percent,
            memory_used_kb: mem.used_kb as i64,
            memory_total_kb: mem.total_kb as i64,
            components_json,
            net_in: net.r#in as i64,
            net_out: net.out as i64,
            load_one: load.one_minute,
            load_five: load.five_minutes,
            load_fifteen: load.fifteen_minutes,
            disks,
            original: metrics,
        });

        // await send for smoothing bursts
        if let Err(e) = self.metric_tx.send(item).await {
            log::error!("[hub] metric queue closed: {e}");
            return Err(Status::unavailable("ingest pipeline unavailable"));
        }
        Ok(())
    }

    async fn upsert_gpus(&self, system_id: i32, gpus: Vec<GpuInfo>) -> Result<(), Status> {
        if gpus.is_empty() {
            return Ok(());
        }

        let mut qb = QueryBuilder::new(
            "INSERT INTO gpus (system_id, gpu_index, uuid, name, pci_bus, driver, memory_total_mb) ",
        );
        qb.push_values(gpus.iter(), |mut b, g| {
            b.push_bind(system_id)
                .push_bind(g.gpu_index as i32)
                .push_bind(&g.uuid)
                .push_bind(&g.name)
                .push_bind(&g.pci_bus)
                .push_bind(&g.driver)
                .push_bind(g.memory_total_mb as i64);
        });
        qb.push(
            " ON CONFLICT (system_id, gpu_index) DO UPDATE SET \
              uuid = EXCLUDED.uuid, name = EXCLUDED.name, pci_bus = EXCLUDED.pci_bus, \
              driver = EXCLUDED.driver, memory_total_mb = EXCLUDED.memory_total_mb",
        );

        qb.build().execute(&self.pool).await.map_err(|e| {
            error!("[hub] GPU upsert error: {e}");
            Status::internal("gpu upsert failed")
        })?;
        Ok(())
    }

    async fn insert_gpu_metrics(
        &self,
        system_id: i32,
        metrics: Vec<GpuMetrics>,
    ) -> Result<(), Status> {
        if metrics.is_empty() {
            return Ok(());
        }
        // Resolve gpu ids in one query
        let idxs: Vec<i32> = metrics.iter().map(|m| m.gpu_index as i32).collect();
        let rows = sqlx::query!(
            "SELECT id, gpu_index FROM gpus WHERE system_id = $1 AND gpu_index = ANY($2)",
            system_id,
            &idxs
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            error!("[hub] GPU id preload error: {e}");
            Status::internal("gpu id preload failed")
        })?;
        let mut id_map = std::collections::HashMap::new();
        for r in rows {
            id_map.insert(r.gpu_index, r.id);
        }

        let mut qb = QueryBuilder::new(
            "INSERT INTO gpu_metrics (gpu_id, time, utilization, memory_used_mb, temperature, power) ",
        );
        let now = Utc::now();
        let mut any = false;
        qb.push_values(
            metrics
                .iter()
                .filter_map(|m| id_map.get(&(m.gpu_index as i32)).map(|gpu_id| (gpu_id, m))),
            |mut b, (gpu_id, m)| {
                any = true;
                b.push_bind(*gpu_id)
                    .push_bind(now)
                    .push_bind(m.utilization)
                    .push_bind(m.memory_used_mb as i64)
                    .push_bind(m.temperature)
                    .push_bind(m.power);
            },
        );
        if !any {
            return Ok(());
        }
        qb.build().execute(&self.pool).await.map_err(|e| {
            error!("[hub] GPU metrics insert error: {e}");
            Status::internal("gpu metrics insert failed")
        })?;
        Ok(())
    }

    async fn upsert_containers(
        &self,
        system_id: i32,
        containers: Vec<ContainerInfo>,
    ) -> Result<(), Status> {
        if containers.is_empty() {
            return Ok(());
        }

        let mut qb =
            QueryBuilder::new("INSERT INTO containers (system_id, docker_id, name, state) ");
        qb.push_values(containers.iter(), |mut b, c| {
            b.push_bind(system_id)
                .push_bind(&c.docker_id)
                .push_bind(&c.name)
                .push_bind(&c.state);
        });
        qb.push(
            " ON CONFLICT (system_id, docker_id) DO UPDATE SET \
              name = EXCLUDED.name, state = EXCLUDED.state",
        );
        qb.build().execute(&self.pool).await.map_err(|e| {
            error!("[hub] Container upsert error: {e}");
            Status::internal("container upsert failed")
        })?;
        Ok(())
    }

    async fn insert_container_metrics(
        &self,
        system_id: i32,
        metrics: Vec<ContainerMetrics>,
    ) -> Result<(), Status> {
        if metrics.is_empty() {
            return Ok(());
        }

        for m in metrics {
            let item = IngestItem::Container(ContainerIngestItem {
                system_id,
                docker_id: m.docker_id.clone(),
                time: Utc::now(),
                cpu_usage: m.cpu_usage,
                memory_usage: m.memory_usage,
                original: m,
            });
            if let Err(e) = self.metric_tx.send(item).await {
                log::error!("[hub] container metric queue closed: {e}");
                return Err(Status::unavailable("ingest pipeline unavailable"));
            }
        }

        /*
        // Collect owned Strings to match expected &\[String]
        let ids: Vec<String> = metrics.iter().map(|m| m.docker_id.clone()).collect();

        let rows = sqlx::query!(
            "SELECT id, docker_id FROM containers WHERE system_id = $1 AND docker_id = ANY($2)",
            system_id,
            &ids
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            error!("[hub] Container id preload error: {e}");
            Status::internal("container id preload failed")
        })?;

        let mut id_map = std::collections::HashMap::new();
        for r in rows {
            id_map.insert(r.docker_id, r.id);
        }

        let mut qb = QueryBuilder::new(
            "INSERT INTO container_metrics (container_id, time, cpu_usage, memory_usage) ",
        );
        let now = Utc::now();
        let mut any = false;
        qb.push_values(
            metrics
                .iter()
                .filter_map(|m| id_map.get(&m.docker_id).map(|cid| (cid, m))),
            |mut b, (cid, m)| {
                any = true;
                b.push_bind(*cid)
                    .push_bind(now)
                    .push_bind(m.cpu_usage)
                    .push_bind(m.memory_usage);
            },
        );
        if !any {
            return Ok(());
        }
        qb.build().execute(&self.pool).await.map_err(|e| {
            error!("[hub] Container metrics insert error: {e}");
            Status::internal("container metrics insert failed")
        })?;*/
        Ok(())
    }
}

#[tonic::async_trait]
impl SystemMonitor for MyMonitor {
    async fn report_metrics(
        &self,
        request: Request<MetricsRequest>,
    ) -> Result<Response<ProtoResponse>, Status> {
        let system_id = self.get_system_id_from_md(request.metadata()).await?;
        let metrics = request.into_inner();
        self.handle_metrics_message(system_id, metrics).await?;
        // record lightweight log in cache
        let cache = self.cache.clone();
        tokio::spawn(async move {
            cache.record_log("info", "metrics inserted").await;
        });
        Ok(Response::new(ProtoResponse {
            status: "200".to_string(),
            message: "Metrics reported successfully".to_string(),
        }))
    }

    async fn stream_metrics(
        &self,
        request: Request<Streaming<MetricsRequest>>,
    ) -> Result<Response<ProtoResponse>, Status> {
        let system_id = self.get_system_id_from_md(request.metadata()).await?;
        let mut inbound = request.into_inner();
        let mut count: u64 = 0;

        while let Some(msg) = inbound.next().await {
            match msg {
                Ok(m) => {
                    if let Err(e) = self.handle_metrics_message(system_id, m).await {
                        return Err(e);
                    }
                    count += 1;
                    if count % 500 == 0 {
                        info!(
                            "[hub] stream_metrics processed {count} messages (system {system_id})"
                        );
                    }
                }
                Err(status) => {
                    log::warn!("[hub] stream_metrics error (system {system_id}): {status}");
                    return Err(Status::aborted("stream receive error"));
                }
            }
        }

        info!("[hub] stream_metrics closed gracefully (system {system_id}, messages={count})");
        Ok(tonic::Response::new(crate::proto::monitor::Response {
            status: "200".into(),
            message: format!("stream closed after {count} messages"),
        }))
    }

    async fn register_gp_us(
        &self,
        request: Request<GpuRequest>,
    ) -> Result<Response<ProtoResponse>, Status> {
        let system_id = self.get_system_id_from_md(request.metadata()).await?;
        let request = request.into_inner();
        self.upsert_gpus(system_id.into(), request.gpus).await?;
        info!("[hub] GPU list updated successfully");
        Ok(Response::new(ProtoResponse {
            status: "200".to_string(),
            message: "GPUs reported successfully".to_string(),
        }))
    }

    async fn report_gpu_metrics(
        &self,
        request: Request<GpuMetricsRequest>,
    ) -> Result<Response<ProtoResponse>, Status> {
        let system_id = self.get_system_id_from_md(request.metadata()).await?;
        let request = request.into_inner();
        self.insert_gpu_metrics(system_id.into(), request.gpu_metrics)
            .await?;
        Ok(Response::new(ProtoResponse {
            status: "200".to_string(),
            message: "GPU metrics reported successfully".to_string(),
        }))
    }

    async fn get_system_info(
        &self,
        request: Request<SystemInfoRequest>,
    ) -> Result<Response<ProtoResponse>, Status> {
        let system_id = self.get_system_id_from_md(request.metadata()).await?;
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
            system_id as i32
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!("[hub] Failed to update system info: {:?}", e);
            Status::internal(format!("Database error: {}", e))
        })?;

        info!("[hub] System info updated successfully");

        Ok(Response::new(ProtoResponse {
            status: "200".to_string(),
            message: "Metrics reported successfully".to_string(),
        }))
    }

    async fn report_systemctl(
        &self,
        request: Request<SystemctlRequest>,
    ) -> Result<Response<ProtoResponse>, Status> {
        let system_id = self.get_system_id_from_md(request.metadata()).await?;
        let request = request.into_inner();
        let services = request.services;
        for service in services {
            // update in-memory cache first for fast reads
            self.cache.upsert_service(service.clone());

            let existing = sqlx::query!(
                r#"SELECT id FROM services WHERE system = $1 AND name = $2"#,
                system_id,
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
                    system_id,
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
        Ok(Response::new(ProtoResponse {
            status: "200".to_string(),
            message: "Services reported successfully".to_string(),
        }))
    }

    async fn register_containers(
        &self,
        request: Request<ContainerRequest>,
    ) -> Result<Response<ProtoResponse>, Status> {
        let system_id = self.get_system_id_from_md(request.metadata()).await?;
        let body = request.into_inner();
        self.upsert_containers(system_id.into(), body.containers)
            .await?;
        Ok(Response::new(ProtoResponse {
            status: "200".to_string(),
            message: "Containers reported successfully".to_string(),
        }))
    }

    async fn report_container_metrics(
        &self,
        request: Request<ContainerMetricsRequest>,
    ) -> Result<Response<ProtoResponse>, Status> {
        let system_id = self.get_system_id_from_md(request.metadata()).await?;
        let body = request.into_inner();
        self.insert_container_metrics(system_id.into(), body.container_metrics)
            .await?;
        Ok(Response::new(ProtoResponse {
            status: "200".to_string(),
            message: "Container metrics successfully".to_string(),
        }))
    }
}
