use crate::lib;
use futures_util::{future, pin_mut, SinkExt, StreamExt, TryStreamExt};
use log::{error, info, warn};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::server::WebPkiClientVerifier;
use rustls::{RootCertStore, ServerConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use sysinfo::System;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpListener;
use tokio::process::{Child, Command};
use tokio::sync::mpsc::{self, channel, Receiver, Sender};
use tokio::sync::{Mutex, Notify};
use tokio::time::interval;
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::tungstenite::error::ProtocolError::{HandshakeIncomplete, WrongHttpMethod};
use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};
use uuid::Uuid;

type ChildHandle = Arc<Mutex<Option<tokio::process::Child>>>;
type ProcessInfo = (ChildHandle, Arc<Notify>);
lazy_static::lazy_static! {
    static ref RUNNING_PROCESSES: Arc<Mutex<HashMap<Uuid, ProcessInfo>>> =
        Arc::new(Mutex::new(HashMap::new()));
    static ref LIVE_METRICS: Arc<Mutex<HashMap<SocketAddr, ProcessInfo>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

type Tx = Sender<Message>;
type Rx = Receiver<Message>;
pub type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")] // This is crucial for enum deserialization
enum WsMessage {
    #[serde(rename = "execute")]
    Execute { command: String, args: Vec<String> },
    #[serde(rename = "stop")]
    Stop,
    #[serde(rename = "update")]
    Update,
    #[serde(rename = "delete")]
    Delete,
    #[serde(rename = "live")]
    Live,
    #[serde(rename = "startservice")]
    StartService {
        service_name: String,
        origin: String,
    },
    #[serde(rename = "stopservice")]
    StopService {
        service_name: String,
        origin: String,
    },
    #[serde(rename = "restartservice")]
    RestartService {
        service_name: String,
        origin: String,
    },
    #[serde(rename = "output")]
    Output(String),
    #[serde(rename = "EOF")]
    EOF,
}

fn load_certs(path: &str) -> Vec<CertificateDer<'static>> {
    let certfile = std::fs::File::open(path).unwrap();
    let mut reader = std::io::BufReader::new(certfile);
    rustls_pemfile::certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}

fn load_private_key(path: &str) -> PrivateKeyDer<'static> {
    let keyfile = std::fs::File::open(path).unwrap();
    let mut reader = std::io::BufReader::new(keyfile);
    rustls_pemfile::pkcs8_private_keys(&mut reader)
        .next()
        .unwrap()
        .unwrap()
        .into()
}

fn load_ca(path: &str) -> RootCertStore {
    let mut ca = RootCertStore::empty();
    let cafile = std::fs::File::open(path).unwrap();
    let mut reader = std::io::BufReader::new(cafile);
    for cert in rustls_pemfile::certs(&mut reader) {
        ca.add(cert.unwrap()).unwrap();
    }
    ca
}

pub async fn stream_output(recp: Tx, child: ChildHandle, terminate_signal: Arc<Notify>) {
    let mut child_opt = child.lock().await;
    if let Some(child) = child_opt.as_mut() {
        let stdout = child
            .stdout
            .take()
            .expect("Child did not have a handle to stdout");
        let stderr = child
            .stderr
            .take()
            .expect("Child did not have a handle to stderr");

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();
        loop {
            tokio::select! {
                Ok(Some(line)) = stdout_reader.next_line() => {
                    //info!("[command:output] {}", line);
                    // Use try_send to avoid blocking and handle full channel
                    if let Err(e) = recp.try_send(Message::Text(Utf8Bytes::from(line))) {
                        info!("[ERROR] Failed to send output: {}", e);
                        break;
                    }
                    // delay for a short period to avoid overwhelming the WebSocket
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
                Ok(Some(line)) = stderr_reader.next_line() => {
                    info!("[command:error] {}", line);
                    if let Err(e) = recp.try_send(Message::Text(Utf8Bytes::from(format!("[ERROR] {}", line)))) {
                        info!("[ERROR] Failed to send error output: {}", e);
                        break;
                    }
                },
                _ = terminate_signal.notified() => {
                    info!("[command] Termination signal received, stopping command");
                    if let Err(e) = child.kill().await {
                        error!("[command] Failed to kill command: {}", e);
                    } else {
                        info!("[command] Command killed successfully");
                    }
                    break;
                },
                 _ = async {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    false
                } => {
                    // This is a timeout to avoid blocking indefinitely
                    if child.try_wait().unwrap().is_some() {
                        info!("[command] Command has exited");
                        if let Err(e) = recp.try_send(Message::Text(Utf8Bytes::from("EOF"))) {
                            info!("[ERROR] Failed to send EOF: {}", e);
                        }
                        break;
                    }
                }
            }
        }
    }
}

pub async fn start_command(command: String, args: Vec<String>, ws_sender: Tx) -> Uuid {
    let process_id = Uuid::new_v4();
    let child = Command::new(&command)
        .args(&args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            let _ = ws_sender.try_send(Message::Text(Utf8Bytes::from(format!(
                "[ERROR] Failed to spawn command: {}",
                e
            ))));
            error!("[ERROR] Failed to spawn command: {}", e);
            e
        })
        .expect("Failed to spawn command");
    let child_handle = Arc::new(Mutex::new(Some(child)));
    let terminate_signal = Arc::new(Notify::new());
    // Store the process information in the global map
    RUNNING_PROCESSES
        .lock()
        .await
        .insert(process_id, (child_handle.clone(), terminate_signal.clone()));

    tokio::spawn(stream_output(ws_sender, child_handle, terminate_signal));

    process_id
}

pub async fn start_metrics_command(addr: SocketAddr, ws_sender: Tx) -> Uuid {
    let process_id = Uuid::new_v4();
    let terminate_signal = Arc::new(Notify::new());
    {
        let terminate_signal = terminate_signal.clone();
        let mut sys = System::new_all();
        let ws_sender = ws_sender.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = terminate_signal.notified() => {
                        info!("[metrics] Termination signal received, stopping live metrics for {}", addr);
                        break;
                    }
                    _ = async {
                        let metrics = lib::system_info::collect_metrics(&mut sys).await;
                        info!("[metrics] Sending live metrics to {}: CPU: {}%, Memory: {}KB used of {}KB ({}%), Load Avg (1m): {}",
                            addr,
                            metrics.cpu_stats.unwrap().usage_percent,
                            metrics.memory_stats.unwrap().used_kb,
                            metrics.memory_stats.unwrap().total_kb,
                            metrics.memory_stats.unwrap().used_kb / metrics.memory_stats.unwrap().total_kb * 100,
                            metrics.load_average.unwrap().one_minute
                        );
                        if let Err(e) = ws_sender.try_send(Message::Text(Utf8Bytes::from(format!(
                            "CPU: {}%, Memory: {}KB used of {}KB ({}%), Load Avg (1m): {}",
                            metrics.cpu_stats.unwrap().usage_percent,
                            metrics.memory_stats.unwrap().used_kb,
                            metrics.memory_stats.unwrap().total_kb,
                            metrics.memory_stats.unwrap().used_kb / metrics.memory_stats.unwrap().total_kb * 100,
                            metrics.load_average.unwrap().one_minute
                        )))) {
                            warn!("[metrics] Failed to send live metrics to {}: {}", addr, e);
                        }
                    } => {}
                }
            }
            let _ = ws_sender.try_send(Message::Text(Utf8Bytes::from("EOF")));
        });
    }

    let child_handle = Arc::new(Mutex::new(None));
    // Store the process information in the global map
    LIVE_METRICS
        .lock()
        .await
        .insert(addr, (child_handle.clone(), terminate_signal.clone()));

    tokio::spawn(stream_output(ws_sender, child_handle, terminate_signal));

    process_id
}

pub async fn start_websocket_server(peers: PeerMap) -> Result<(), Box<dyn std::error::Error>> {
    let addr = env::var("LYNX_AGENT_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let cert_path = env::var("LYNX_CERT_PATH").unwrap_or_else(|_| "certs/agent.crt".to_string());
    let key_path = env::var("LYNX_KEY_PATH").unwrap_or_else(|_| "certs/agent.key".to_string());
    let ca_path = env::var("LYNX_CA_PATH").unwrap_or_else(|_| "certs/ca.crt".to_string());
    let certs = load_certs(&cert_path);
    let key = load_private_key(&key_path);
    let ca_store = load_ca(&ca_path);

    let config = ServerConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
        .with_client_cert_verifier(
            WebPkiClientVerifier::builder(Arc::new(ca_store))
                .build()
                .map_err(|e| format!("Failed to build client cert verifier: {}", e))?,
        )
        .with_single_cert(certs, key)?;

    let acceptor = TlsAcceptor::from(Arc::new(config));

    info!("[agent] Started mTLS websocket server at {}", addr);

    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    let peers_clone = peers.clone();

    tokio::spawn(async move {
        while let Ok((stream, addr)) = listener.accept().await {
            let acceptor = acceptor.clone();
            let peers_clone = peers_clone.clone();
            tokio::spawn(async move {
                let tls_stream = match acceptor.accept(stream).await {
                    Ok(tls_stream) => tls_stream,
                    Err(e) => {
                        error!(
                            "[mTLS] Failed to establish TLS connection with {}: {}",
                            addr, e
                        );
                        return;
                    }
                };

                let ws_stream = match tokio_tungstenite::accept_async(tls_stream).await {
                    Ok(ws_stream) => ws_stream,
                    Err(e) => {
                        error!(
                            "[ws] Failed to accept WebSocket connection from {}: {}",
                            addr, e
                        );
                        return;
                    }
                };

                info!("[ws] mTLS connection established: {}", addr);
                let (tx, mut rx) = channel(64);
                peers_clone.lock().await.insert(addr, tx.clone());

                let (mut outgoing, incoming) = ws_stream.split();
                // Process incoming messages
                let incoming_messages = incoming.try_for_each(|msg| {
                    if let Ok(text) = msg.to_text() {
                        info!("[ws] Received message from {}: {}", addr, text);
                        match serde_json::from_str::<WsMessage>(text) {
                            Ok(WsMessage::Execute { command, args }) => {
                                info!("[ws] Executing command: {} {:?}", command, args);
                                let tx_clone = tx.clone();
                                tokio::spawn(async move {
                                    let process_id =
                                        start_command(command, args, tx_clone.clone()).await;
                                    let _ = tx_clone.try_send(Message::Text(Utf8Bytes::from(
                                        format!("Started command with ID: {}", process_id),
                                    )));
                                });
                            }
                            Ok(WsMessage::Stop) => {
                                let tx_clone = tx.clone();
                                tokio::spawn(async move {
                                    let mut processes = RUNNING_PROCESSES.lock().await;
                                    for (pid, (child_handle, terminate_signal)) in
                                        processes.clone().iter()
                                    {
                                        terminate_signal.notify_one();
                                        if let Some(child) = child_handle.lock().await.as_mut() {
                                            if let Err(e) = child.kill().await {
                                                info!("[ws] Failed to stop command {}: {}", pid, e);
                                                let _ = tx_clone.try_send(Message::Text(
                                                    Utf8Bytes::from(format!(
                                                        "Failed to stop command {}: {}",
                                                        pid, e
                                                    )),
                                                ));
                                            } else {
                                                let _ = tx_clone.try_send(Message::Text(
                                                    Utf8Bytes::from(format!(
                                                        "Stopped command {}",
                                                        pid
                                                    )),
                                                ));
                                            }
                                        } else {
                                            continue;
                                        }
                                        processes.remove(pid);
                                    }
                                });
                            }
                            Ok(WsMessage::Update) => {
                                // todo: Make update script
                            }
                            Ok(WsMessage::Delete) => {
                                // todo: Uninstall self
                            }
                            Ok(WsMessage::Live) => {
                                info!(
                                    "[ws] Starting live relay of system metrics to agent: {}",
                                    addr
                                );
                                let tx_clone = tx.clone();
                                tokio::spawn(async move {
                                    let process_id =
                                        start_metrics_command(addr, tx_clone.clone()).await;
                                    let _ =
                                        tx_clone.try_send(Message::Text(Utf8Bytes::from(format!(
                                            "Started live metrics with thread ID: {}",
                                            process_id
                                        ))));
                                });
                            }
                            Ok(WsMessage::StartService {
                                service_name,
                                origin,
                            }) => {
                                let systemctl = systemctl::SystemCtl::default();
                                let tx_clone = tx.clone();
                                tokio::spawn(async move {
                                    if origin == "systemctl" {
                                        match systemctl.start(&service_name) {
                                            Ok(_) => {
                                                let _ = tx_clone.try_send(Message::Text(
                                                    Utf8Bytes::from(format!(
                                                        "Started service: {}",
                                                        service_name
                                                    )),
                                                ));
                                            }
                                            Err(e) => {
                                                let _ = tx_clone.try_send(Message::Text(
                                                    Utf8Bytes::from(format!(
                                                        "Failed to start service {}: {}",
                                                        service_name, e
                                                    )),
                                                ));
                                            }
                                        }
                                    } else if origin == "docker" {
                                        let docker_manager = lib::docker::DockerManager::new()
                                            .map_err(|e| {
                                                let _ = tx_clone.try_send(Message::Text(
                                                    Utf8Bytes::from(format!(
                                                        "Failed to start docker manager: {}",
                                                        e
                                                    )),
                                                ));
                                            })
                                            .unwrap();

                                        match docker_manager.start_container(&service_name).await {
                                            Ok(_) => {
                                                let _ = tx_clone.try_send(Message::Text(
                                                    Utf8Bytes::from(format!(
                                                        "Started docker container: {}",
                                                        service_name
                                                    )),
                                                ));
                                            }
                                            Err(e) => {
                                                let _ = tx_clone.try_send(Message::Text(
                                                    Utf8Bytes::from(format!(
                                                        "Failed to start docker container: {}",
                                                        e
                                                    )),
                                                ));
                                            }
                                        }
                                    }
                                });
                            }
                            Ok(WsMessage::StopService {
                                service_name,
                                origin,
                            }) => {
                                let systemctl = systemctl::SystemCtl::default();
                                let tx_clone = tx.clone();
                                tokio::spawn(async move {
                                    if origin == "systemctl" {
                                        match systemctl.stop(&service_name) {
                                            Ok(_) => {
                                                let _ = tx_clone.try_send(Message::Text(
                                                    Utf8Bytes::from(format!(
                                                        "Stopped service: {}",
                                                        service_name
                                                    )),
                                                ));
                                            }
                                            Err(e) => {
                                                let _ = tx_clone.try_send(Message::Text(
                                                    Utf8Bytes::from(format!(
                                                        "Failed to stop service {}: {}",
                                                        service_name, e
                                                    )),
                                                ));
                                            }
                                        }
                                    } else if origin == "docker" {
                                        let docker_manager = lib::docker::DockerManager::new()
                                            .map_err(|e| {
                                                let _ = tx_clone.try_send(Message::Text(
                                                    Utf8Bytes::from(format!(
                                                        "Failed to start docker manager: {}",
                                                        e
                                                    )),
                                                ));
                                            })
                                            .unwrap();

                                        match docker_manager.stop_container(&service_name).await {
                                            Ok(_) => {
                                                let _ = tx_clone.try_send(Message::Text(
                                                    Utf8Bytes::from(format!(
                                                        "Stopped docker container: {}",
                                                        service_name
                                                    )),
                                                ));
                                            }
                                            Err(e) => {
                                                let _ = tx_clone.try_send(Message::Text(
                                                    Utf8Bytes::from(format!(
                                                        "Failed to stop docker container: {}",
                                                        e
                                                    )),
                                                ));
                                            }
                                        }
                                    }
                                });
                            }
                            Ok(WsMessage::RestartService {
                                service_name,
                                origin,
                            }) => {
                                let systemctl = systemctl::SystemCtl::default();
                                let tx_clone = tx.clone();
                                tokio::spawn(async move {
                                    if origin == "systemctl" {
                                        match systemctl.restart(&service_name) {
                                            Ok(status) => {
                                                let _ = tx_clone.try_send(Message::Text(
                                                    Utf8Bytes::from(format!(
                                                        "Restarted service: {}",
                                                        status
                                                    )),
                                                ));
                                            }
                                            Err(e) => {
                                                let _ = tx_clone.try_send(Message::Text(
                                                    Utf8Bytes::from(format!(
                                                        "Failed to restart service {}: {}",
                                                        service_name, e
                                                    )),
                                                ));
                                            }
                                        }
                                    } else if origin == "docker" {
                                        let docker_manager = lib::docker::DockerManager::new()
                                            .map_err(|e| {
                                                let _ = tx_clone.try_send(Message::Text(
                                                    Utf8Bytes::from(format!(
                                                        "Failed to start docker manager: {}",
                                                        e
                                                    )),
                                                ));
                                            })
                                            .unwrap();

                                        match docker_manager.restart_container(&service_name).await
                                        {
                                            Ok(_) => {
                                                let _ = tx_clone.try_send(Message::Text(
                                                    Utf8Bytes::from(format!(
                                                        "Restarted docker container: {}",
                                                        service_name
                                                    )),
                                                ));
                                            }
                                            Err(e) => {
                                                let _ = tx_clone.try_send(Message::Text(
                                                    Utf8Bytes::from(format!(
                                                        "Failed to restart docker container: {}",
                                                        e
                                                    )),
                                                ));
                                            }
                                        }
                                    } else {
                                        let _ = tx_clone.try_send(Message::Text(Utf8Bytes::from(
                                            format!("Invalid origin for service command"),
                                        )));
                                    }
                                });
                            }
                            Ok(WsMessage::EOF) | Err(_) | _ => {
                                let peers_thread = peers_clone.clone();
                                tokio::spawn(async move {
                                    peers_thread.lock().await.remove(&addr);
                                });
                                return future::err(
                                    tokio_tungstenite::tungstenite::Error::Protocol(
                                        HandshakeIncomplete,
                                    ),
                                );
                            }
                        }
                    }
                    future::ok(())
                });

                // Forward messages from rx thread to outgoing websocket stream
                let outgoing_messages = async move {
                    while let Some(msg) = rx.recv().await {
                        if let Err(e) = outgoing.send(msg).await {
                            error!("[ws] Failed to send message to {}: {}", addr, e);
                            break;
                        }
                    }
                };

                // Run both tasks concurrently
                tokio::select! {
                    _ = incoming_messages => {},
                    _ = outgoing_messages => {},
                }

                info!("{} disconnected", &addr);
                peers_clone.lock().await.remove(&addr);
                tokio::spawn(async move {
                    let mut live_metrics = LIVE_METRICS.lock().await;
                    if let Some((child_handle, terminate_signal)) = live_metrics.remove(&addr) {
                        info!("[ws] Stopping live metrics for {}", addr);
                        terminate_signal.notify_one();
                        if let Some(child) = child_handle.lock().await.as_mut() {
                            if let Err(e) = child.kill().await {
                                info!("[ws] Failed to stop live metrics for {}: {}", addr, e);
                            } else {
                                info!("[ws] Stopped live metrics for {}", addr);
                            }
                        }
                    }
                });
            });
        }
    });
    Ok(())
}
