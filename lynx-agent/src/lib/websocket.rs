use futures_util::{SinkExt, StreamExt, TryStreamExt, future, pin_mut};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpListener;
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, Notify};
use tokio::sync::mpsc::{self, Sender, Receiver, channel};

use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};
use tokio_tungstenite::tungstenite::error::ProtocolError::{HandshakeIncomplete, WrongHttpMethod};
use uuid::Uuid;
type ChildHandle = Arc<Mutex<tokio::process::Child>>;
type ProcessInfo = (ChildHandle, Arc<Notify>);
lazy_static::lazy_static! {
    static ref RUNNING_PROCESSES: Arc<Mutex<HashMap<Uuid, ProcessInfo>>> =
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
    #[serde(rename = "output")]
    Output(String),
    #[serde(rename = "EOF")]
    EOF,
}

pub async fn stream_output(
    recp: Tx,
    command: String,
    args: Vec<String>,
    child: ChildHandle,
    terminate_signal: Arc<Notify>,
) {
    let mut child = child.lock().await;
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
    drop(child); // Ensure the child process is dropped (prob dont need but :shrug:)
}

pub async fn start_command(command: String, args: Vec<String>, ws_sender: Tx) -> Uuid {
    let process_id = Uuid::new_v4();
    let child = Command::new(&command)
        .args(&args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            let _ = ws_sender.try_send(Message::Text(Utf8Bytes::from(format!("[ERROR] Failed to spawn command: {}", e))));
            error!("[ERROR] Failed to spawn command: {}", e);
            e
        })
        .expect("Failed to spawn command");
    let child_handle = Arc::new(Mutex::new(child));
    let terminate_signal = Arc::new(Notify::new());
    // Store the process information in the global map
    RUNNING_PROCESSES
        .lock()
        .await
        .insert(process_id, (child_handle.clone(), terminate_signal.clone()));

    tokio::spawn(stream_output(
        ws_sender,
        command,
        args,
        child_handle,
        terminate_signal,
    ));

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
                                let process_id = start_command(command, args, tx_clone.clone()).await;
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
                                for (pid, (child_handle, terminate_signal)) in processes.clone().iter() {
                                    terminate_signal.notify_one();
                                    if let Err(e) = child_handle.lock().await.kill().await {
                                        info!("[ws] Failed to stop command {}: {}", pid, e);
                                        let _ = tx_clone.try_send(Message::Text(Utf8Bytes::from(
                                            format!("Failed to stop command {}: {}", pid, e),
                                        )));
                                    } else {
                                        let _ = tx_clone.try_send(Message::Text(Utf8Bytes::from(
                                            format!("Stopped command {}", pid),
                                        )));
                                    }
                                    processes.remove(pid);
                                }
                            });
                        }
                        Ok(WsMessage::EOF) | Err(_) | _ => {
                            let peers_thread = peers_clone.clone();
                            tokio::spawn(async move {
                                peers_thread.lock().await.remove(&addr);
                            });
                            return future::err(tokio_tungstenite::tungstenite::Error::Protocol(
                                HandshakeIncomplete
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
        }
    });

    Ok(())
}
