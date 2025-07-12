use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use tokio::process::{Command, Child};
use tokio::sync::{Mutex};
use std::sync::{Arc};
use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, SinkExt, StreamExt, TryStreamExt};
use log::info;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpListener;

use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};
use uuid::Uuid;

lazy_static::lazy_static! {
    static ref RUNNING_PROCESSES: Arc<Mutex<HashMap<Uuid, Child>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;
#[derive(Serialize, Deserialize)]
enum WsMessage {
    Execute { command: String, args: Vec<String> },
    Stop,
    Output(String),
}

pub async fn stream_output(recp: Tx, command: String, args: Vec<String>) {
    let mut child = Command::new(command)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start command");

    let stdout = child.stdout.take().expect("Child did not have a handle to stdout");
    let stderr = child.stderr.take().expect("Child did not have a handle to stderr");

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    loop {
        tokio::select! {
            Ok(Some(line)) = stdout_reader.next_line() => {
                recp.unbounded_send(Message::Text(Utf8Bytes::from(line))).unwrap();
            }
            Ok(Some(line)) = stderr_reader.next_line() => {
                recp.unbounded_send(Message::Text(Utf8Bytes::from(format!("[ERROR] {}", line)))).unwrap();
            }
            else => break,
        }
    }
}

pub async fn start_command(
    command: String,
    args: Vec<String>,
    ws_sender: Tx,
) -> Uuid {
    let process_id = Uuid::new_v4();
    let mut child = Command::new(&command)
        .args(&args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start command");

    RUNNING_PROCESSES.lock().await.insert(process_id, child);
    
    tokio::spawn(stream_output(ws_sender, command, args));
    
    process_id
}

pub async fn start_websocket_server(peers: PeerMap) -> Result<(), Box<dyn std::error::Error>> {
    let addr = env::var("LYNX_AGENT_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    info!("[agent] Started websocket server at {}", addr);
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    let state_clone = peers.clone();
    tokio::spawn(async move {
        while let Ok((stream, addr)) = listener.accept().await {
            let ws_stream = tokio_tungstenite::accept_async(stream).await
                .expect("Failed to accept");

            info!("[ws] Connection established: {}", addr);
            let (tx, rx) = unbounded();
            state_clone.lock().await.insert(addr, tx.clone());

            let (mut outgoing, mut incoming) = ws_stream.split();

            while let Some(msg) = incoming.next().await {
                if let Ok(msg) = msg {
                    if let Ok(text) = msg.to_text() {
                        if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(text) {
                            match ws_msg {
                                WsMessage::Execute { command, args } => {
                                    info!("Executing command: {} {:?}", command, args);
                                    let process_id = start_command(command, args, tx.clone()).await;
                                    outgoing.send(Message::Text(Utf8Bytes::from(format!("Started process with ID: {}", process_id)))).await.unwrap();
                                }
                                WsMessage::Stop => {
                                    info!("Stopping all processes");
                                    let mut processes = RUNNING_PROCESSES.lock().await;
                                    for (id, child) in processes.iter_mut() {
                                        if let Err(e) = child.kill().await {
                                            info!("Failed to stop process {}: {}", id, e);
                                        } else {
                                            info!("Stopped process {}", id);
                                        }
                                    }
                                    processes.clear();
                                    outgoing.send(Message::Text(Utf8Bytes::from("All processes stopped"))).await.unwrap();
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            info!("{} disconnected", &addr);
            state_clone.lock().await.remove(&addr);
        }
    });
    
    Ok(())
}