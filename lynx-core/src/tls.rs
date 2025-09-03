use std::error::Error;
use std::fs;
use std::path::Path;
use tonic::transport::{Certificate, Identity, ServerTlsConfig};

pub fn build_tls_config(certs_dir: &Path) -> Result<ServerTlsConfig, Box<dyn Error>> {
    if !certs_dir.exists() {
        return Err(format!("Certificates directory not found: {:?}", certs_dir).into());
    }

    let server_cert_path = certs_dir.join("server.crt");
    let server_key_path = certs_dir.join("server.key");
    if !server_cert_path.exists() || !server_key_path.exists() {
        return Err(format!("Server certificate or key not found in {:?}", certs_dir).into());
    }

    let ca_cert_path = certs_dir.join("ca.crt");
    if !ca_cert_path.exists() {
        return Err(format!("CA certificate not found in {:?}", certs_dir).into());
    }

    let server_cert = fs::read_to_string(&server_cert_path)?;
    let server_key = fs::read_to_string(&server_key_path)?;
    if server_cert.is_empty() || server_key.is_empty() {
        return Err("Server certificate or key is empty".into());
    }
    let ca_cert = fs::read_to_string(&ca_cert_path)?;
    let tls = ServerTlsConfig::new()
        .identity(Identity::from_pem(server_cert, server_key))
        .client_ca_root(Certificate::from_pem(ca_cert))
        .client_auth_optional(false);
    Ok(tls)
}
