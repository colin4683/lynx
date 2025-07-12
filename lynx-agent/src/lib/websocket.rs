use futures_channel::mpsc::{UnboundedSender, unbounded};
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

use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};
use tokio_tungstenite::tungstenite::error::ProtocolError::{HandshakeIncomplete, WrongHttpMethod};
use uuid::Uuid;
type ChildHandle = Arc<Mutex<tokio::process::Child>>;
type ProcessInfo = (ChildHandle, Arc<Notify>);
lazy_static::lazy_static! {
    static ref RUNNING_PROCESSES: Arc<Mutex<HashMap<Uuid, ProcessInfo>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;
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
                recp.unbounded_send(Message::Text(Utf8Bytes::from(line))).map_err(
                    |e| info!("[ERROR] Failed to send output: {}", e)
                ).unwrap();
                // delay for a short period to avoid overwhelming the WebSocket
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
            Ok(Some(line)) = stderr_reader.next_line() => {
                info!("[command:error] {}", line);
                recp.unbounded_send(Message::Text(Utf8Bytes::from(format!("[ERROR] {}", line)))).unwrap();
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
                    recp.unbounded_send(Message::Text(Utf8Bytes::from("EOF"))).unwrap();
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
            ws_sender.unbounded_send(Message::Text(Utf8Bytes::from(format!("[ERROR] Failed to spawn command: {}", e)))).unwrap();
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
            let (tx, rx) = unbounded(); // Channels for current connection
            peers_clone.lock().await.insert(addr, tx.clone());

            let (outgoing, incoming) = ws_stream.split();
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
                                tx_clone
                                    .unbounded_send(Message::Text(Utf8Bytes::from(format!(
                                        "Started command with ID: {}",
                                        process_id
                                    ))))
                                    .unwrap();
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
                                    if let Err(e) = child_handle.lock().await.kill().await {
                                        info!("[ws] Failed to stop command {}: {}", pid, e);
                                        tx_clone
                                            .unbounded_send(Message::Text(Utf8Bytes::from(
                                                format!("Failed to stop command {}: {}", pid, e),
                                            )))
                                            .unwrap();
                                    } else {
                                        tx_clone
                                            .unbounded_send(Message::Text(Utf8Bytes::from(
                                                format!("Stopped command {}", pid),
                                            )))
                                            .unwrap();
                                    }
                                    
                                    processes.remove(pid);
                                }
                            });
                        }
                        Ok(WsMessage::EOF) => {
                            let peers_thread = peers_clone.clone();
                            tokio::spawn(async move {
                                peers_thread.lock().await.remove(&addr);
                            });
                            return future::err(tokio_tungstenite::tungstenite::Error::Protocol(
                                HandshakeIncomplete
                            ));
                        }
                        Err(e) => {
                            let peers_thread = peers_clone.clone();
                            tokio::spawn(async move {
                                peers_thread.lock().await.remove(&addr);
                            });
                            return future::err(tokio_tungstenite::tungstenite::Error::Protocol(
                                HandshakeIncomplete
                            ));
                        }
                        _ => {
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
            let outgoing_messages = rx.map(Ok).forward(outgoing);

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
