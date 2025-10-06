use crate::lib::collectors::CollectorRequest;
use crate::proto;
use crate::proto::monitor::system_monitor_client::SystemMonitorClient;
use log::{error, info};
use serde::Deserialize;
use std::fs;
use std::time::Duration;
use tokio::time::timeout;
use tonic::codegen::InterceptedService;
use tonic::metadata::MetadataValue;
use tonic::service::Interceptor;
use tonic::transport::{Certificate, ClientTlsConfig, Identity};
use tonic::{Code, Status};

pub async fn tls_config() -> Result<ClientTlsConfig, Box<dyn std::error::Error>> {
    let current_dir = std::env::current_dir()?;
    let certs_dir = current_dir.join("certs");

    if !certs_dir.exists() {
        return Err(format!("Certificates directory not found: {:?}", certs_dir).into());
    }

    let client_cert_path = certs_dir.join("docker-agent.crt");
    let client_key_path = certs_dir.join("docker-agent.key");

    if !client_cert_path.exists() || !client_key_path.exists() {
        return Err(format!("Client certificate or key not found in {:?}", certs_dir).into());
    }

    let ca_cert_path = certs_dir.join("ca.crt");
    if !ca_cert_path.exists() {
        return Err(format!("CA certificate not found in {:?}", certs_dir).into());
    }

    let client_cert = fs::read_to_string(client_cert_path)?;
    let client_key = fs::read_to_string(client_key_path)?;
    let ca_cert = fs::read_to_string(ca_cert_path)?;

    if client_cert.is_empty() || client_key.is_empty() || ca_cert.is_empty() {
        return Err("Client certificate, key, or CA certificate is empty".into());
    }

    let client_tls_config = ClientTlsConfig::new()
        .ca_certificate(Certificate::from_pem(ca_cert.as_bytes()))
        .identity(Identity::from_pem(
            client_cert.as_bytes(),
            client_key.as_bytes(),
        ));

    Ok(client_tls_config)
}

#[derive(Deserialize, Debug)]
pub struct CoreConfig {
    pub server_url: String,
    pub agent_key: String,
}

#[derive(Deserialize, Debug)]
pub struct LynxConfig {
    pub core: CoreConfig,
}

pub struct AuthInterceptor {
    pub agent_key: String,
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

pub struct GrpcClient {
    client: SystemMonitorClient<InterceptedService<tonic::transport::Channel, AuthInterceptor>>,
    config: LynxConfig,
    client_tls_config: tonic::transport::ClientTlsConfig,
}

impl GrpcClient {
    pub fn new(
        client: SystemMonitorClient<InterceptedService<tonic::transport::Channel, AuthInterceptor>>,
        config: LynxConfig,
        client_tls_config: tonic::transport::ClientTlsConfig,
    ) -> Self {
        Self {
            client,
            config,
            client_tls_config,
        }
    }

    pub async fn send_request<T, F>(
        &mut self,
        request: T,
        operation: F,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        F: for<'a> FnOnce(
            &'a mut SystemMonitorClient<
                InterceptedService<tonic::transport::Channel, AuthInterceptor>,
            >,
            tonic::Request<T>,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<tonic::Response<proto::monitor::Response>, tonic::Status>,
                    > + Send
                    + 'a,
            >,
        >,
    {
        let rpc_timeout = Duration::from_secs(10);
        let request = tonic::Request::new(request);

        match timeout(rpc_timeout, operation(&mut self.client, request)).await {
            Ok(Ok(response)) => {
                let resp = response.into_inner();
                if resp.status == "200" {
                    info!("[agent] Request successful");
                } else {
                    info!("[agent] Request failed: {:?}", resp.message);
                }
                Ok(())
            }
            Ok(Err(e)) => {
                error!("[agent] Error sending request: {}", e);
                if e.code() == Code::Unavailable || e.code() == Code::DeadlineExceeded {
                    self.reconnect().await?;
                }
                Ok(())
            }
            Err(_) => {
                error!("[agent] Request timeout; reconnecting");
                self.reconnect().await?;
                Ok(())
            }
        }
    }

    async fn reconnect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let make_client =
            |config: &LynxConfig,
             tls: tonic::transport::ClientTlsConfig|
             -> Result<tonic::transport::Endpoint, Box<dyn std::error::Error>> {
                let endpoint =
                    tonic::transport::Endpoint::from_shared(config.core.server_url.clone())?
                        .tls_config(tls)?
                        .tcp_keepalive(Some(Duration::from_secs(30)))
                        .http2_keep_alive_interval(Duration::from_secs(15))
                        .keep_alive_timeout(Duration::from_secs(5))
                        .keep_alive_while_idle(true)
                        .connect_timeout(Duration::from_secs(10));
                Ok(endpoint)
            };
        let endpoint = make_client(&self.config, self.client_tls_config.clone())?;
        let channel = endpoint.connect().await?;
        self.client = SystemMonitorClient::with_interceptor(
            channel,
            AuthInterceptor {
                agent_key: self.config.core.agent_key.clone(),
            },
        );
        Ok(())
    }
}

pub async fn handle_collector_requests(
    grpc_client: &mut GrpcClient,
    request: CollectorRequest,
) -> Result<(), Box<dyn std::error::Error>> {
    match request {
        CollectorRequest::SystemInfo(info) => {
            info!("[agent] Sending system info to hub...");
            grpc_client
                .send_request(info, move |client, req| {
                    Box::pin(client.get_system_info(req))
                })
                .await
        }
        CollectorRequest::Metrics(metrics) => {
            info!("[agent] Sending metrics to hub...");
            grpc_client
                .send_request(metrics, move |client, req| {
                    Box::pin(client.report_metrics(req))
                })
                .await
        }
        CollectorRequest::Systemctl(systemctl) => {
            info!("[agent] Sending systemctl services to hub...");
            grpc_client
                .send_request(systemctl, move |client, req| {
                    Box::pin(client.report_systemctl(req))
                })
                .await
        }
        CollectorRequest::GpuInfo(gpu_info) => {
            info!("[agent] Sending GPU info to hub...");
            grpc_client
                .send_request(gpu_info, move |client, req| {
                    Box::pin(client.register_gp_us(req))
                })
                .await
        }
        CollectorRequest::GpuMetrics(gpu_metrics) => {
            info!("[agent] Sending GPU metrics to hub...");
            grpc_client
                .send_request(gpu_metrics, move |client, req| {
                    Box::pin(client.report_gpu_metrics(req))
                })
                .await
        }
        CollectorRequest::ContainerInfo(container_info) => {
            info!("[agent] Sending container info to hub...");
            grpc_client
                .send_request(container_info, move |client, req| {
                    Box::pin(client.register_containers(req))
                })
                .await
        }
        CollectorRequest::ContainerMetrics(container_metrics) => {
            info!("[agent] Sending container metrics to hub...");
            grpc_client
                .send_request(container_metrics, move |client, req| {
                    Box::pin(client.report_container_metrics(req))
                })
                .await
        }
    }
}
