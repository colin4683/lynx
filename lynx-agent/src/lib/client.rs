use std::fs;
use tonic::transport::{Certificate, ClientTlsConfig, Identity};

pub async fn tls_config() -> Result<ClientTlsConfig, Box<dyn std::error::Error>> {
    let current_dir = std::env::current_dir()?;
    let certs_dir = current_dir.join("certs");
    
    if !certs_dir.exists() {
        return Err(format!("Certificates directory not found: {:?}", certs_dir).into());
    }
    
    let client_cert_path = certs_dir.join("agent.crt");
    let client_key_path = certs_dir.join("agent.key");
    
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
        .identity(Identity::from_pem(client_cert.as_bytes(), client_key.as_bytes()));
    
    
    Ok(client_tls_config)
}