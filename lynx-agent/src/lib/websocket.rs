use crate::lib;
use futures_util::{future, pin_mut, SinkExt, StreamExt, TryStreamExt};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
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
    StartService { service_name: String },
    #[serde(rename = "stopservice")]
    StopService { service_name: String },
    #[serde(rename = "restartservice")]
    RestartService { service_name: String },
    #[serde(rename = "output")]
    Output(String),
    #[serde(rename = "EOF")]
    EOF,
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
    info!("[agent] Started websocket server at {}", addr);
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    let peers_clone = peers.clone();
    tokio::spawn(async move {
        while let Ok((stream, addr)) = listener.accept().await {
            let ws_stream = tokio_tungstenite::accept_async(stream)
                .await
                .expect("Failed to accept");

            info!("[ws] Connection established: {}", addr);
            // Use a bounded channel (e.g., 64 messages) to avoid memory leaks
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
                                let _ = tx_clone.try_send(Message::Text(Utf8Bytes::from(format!(
                                    "Started command with ID: {}",
                                    process_id
                                ))));
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
                                                Utf8Bytes::from(format!("Stopped command {}", pid)),
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
                                let _ = tx_clone.try_send(Message::Text(Utf8Bytes::from(format!(
                                    "Started live metrics with thread ID: {}",
                                    process_id
                                ))));
                            });
                        }
                        Ok(WsMessage::StartService { service_name }) => {
                            let systemctl = systemctl::SystemCtl::default();
                            let tx_clone = tx.clone();
                            tokio::spawn(async move {
                                match systemctl.start(&service_name) {
                                    Ok(_) => {
                                        let _ = tx_clone.try_send(Message::Text(Utf8Bytes::from(
                                            format!("Started service: {}", service_name),
                                        )));
                                    }
                                    Err(e) => {
                                        let _ = tx_clone.try_send(Message::Text(Utf8Bytes::from(
                                            format!(
                                                "Failed to start service {}: {}",
                                                service_name, e
                                            ),
                                        )));
                                    }
                                }
                            });
                        }
                        Ok(WsMessage::StopService { service_name }) => {
                            let systemctl = systemctl::SystemCtl::default();
                            let tx_clone = tx.clone();
                            tokio::spawn(async move {
                                match systemctl.stop(&service_name) {
                                    Ok(_) => {
                                        let _ = tx_clone.try_send(Message::Text(Utf8Bytes::from(
                                            format!("Stopped service: {}", service_name),
                                        )));
                                    }
                                    Err(e) => {
                                        let _ = tx_clone.try_send(Message::Text(Utf8Bytes::from(
                                            format!(
                                                "Failed to stop service {}: {}",
                                                service_name, e
                                            ),
                                        )));
                                    }
                                }
                            });
                        }
                        Ok(WsMessage::RestartService { service_name }) => {
                            let systemctl = systemctl::SystemCtl::default();
                            let tx_clone = tx.clone();
                            tokio::spawn(async move {
                                match systemctl.restart(&service_name) {
                                    Ok(status) => {
                                        let _ = tx_clone.try_send(Message::Text(Utf8Bytes::from(
                                            format!("Restarted service: {}", status),
                                        )));
                                    }
                                    Err(e) => {
                                        let _ = tx_clone.try_send(Message::Text(Utf8Bytes::from(
                                            format!(
                                                "Failed to restart service {}: {}",
                                                service_name, e
                                            ),
                                        )));
                                    }
                                }
                            });
                        }
                        Ok(WsMessage::EOF) | Err(_) | _ => {
                            let peers_thread = peers_clone.clone();
                            tokio::spawn(async move {
                                peers_thread.lock().await.remove(&addr);
                            });
                            return future::err(tokio_tungstenite::tungstenite::Error::Protocol(
                                HandshakeIncomplete,
                            ));
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
        }
    });
    Ok(())
}
