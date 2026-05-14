use futures_util::{SinkExt, StreamExt};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::{ClientConfig, RootCertStore};
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};

fn load_certs(path: &str) -> Vec<CertificateDer<'static>> {
    println!("loading certs from {}", path);
    let certfile = File::open(path).unwrap();
    let mut reader = BufReader::new(certfile);
    rustls_pemfile::certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}

fn load_private_key(path: &str) -> PrivateKeyDer<'static> {
    let keyfile = File::open(path).unwrap();
    let mut reader = BufReader::new(keyfile);
    rustls_pemfile::pkcs8_private_keys(&mut reader)
        .next()
        .unwrap()
        .unwrap()
        .into()
}

fn load_ca(path: &str) -> RootCertStore {
    let mut ca = RootCertStore::empty();
    let cafile = File::open(path).unwrap();
    let mut reader = BufReader::new(cafile);
    for cert in rustls_pemfile::certs(&mut reader) {
        ca.add(cert.unwrap()).unwrap();
    }
    ca
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let current_dir = std::env::current_dir()?;
    let certs_dir = current_dir.join("certs");
    let cert_path = env::var("LYNX_CERT_PATH").unwrap_or_else(|_| "certs/agent.crt".to_string());
    let key_path = env::var("LYNX_KEY_PATH").unwrap_or_else(|_| "certs/agent.key".to_string());
    let ca_path = env::var("LYNX_CA_PATH").unwrap_or_else(|_| "certs/ca.crt".to_string());
    let certs = load_certs(&cert_path);
    let key = load_private_key(&key_path);
    let ca_store = load_ca(&ca_path);

    let config = ClientConfig::builder()
        .with_root_certificates(ca_store)
        .with_client_auth_cert(certs, key)?;

    let connector = tokio_rustls::TlsConnector::from(Arc::new(config));

    // Parse URL for connection details
    let url = url::Url::parse("wss://127.0.0.1:8080")?;
    let host = url.host_str().unwrap().to_string();
    let port = url.port().unwrap_or(8080);
    let url_str = url.as_str().to_string();

    // Manually connect to the socket first
    let tcp_stream = tokio::net::TcpStream::connect((host.as_str(), port)).await?;

    // Perform TLS handshake
    let domain = rustls::pki_types::ServerName::try_from(host.clone())?;
    let tls_stream = connector.connect(domain, tcp_stream).await?;

    // Perform WebSocket handshake
    let (ws_stream, _) = tokio_tungstenite::client_async(&url_str, tls_stream).await?;

    println!("Connected to WebSocket server");

    let (mut write, mut read) = ws_stream.split();

    // Send a test message
    let test_msg = r#"{"type":"execute","command":"echo","args":["Hello from test client"]}"#;
    write.send(Message::Text(Utf8Bytes::from(test_msg))).await?;

    // Read responses
    while let Some(msg) = read.next().await {
        match msg? {
            Message::Text(text) => println!("Received: {}", text),
            Message::Close(_) => break,
            _ => {}
        }
    }

    Ok(())
}
