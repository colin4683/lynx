use lynx_core::tls::build_tls_config;
use std::path::PathBuf;

#[test]
fn tls_config_builds_with_valid_certs() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("certs");
    let cfg = build_tls_config(&path).expect("TLS config should build with provided certs");
    // Can't easily assert internals; presence is success.
    let _ = cfg;
}

#[test]
fn tls_config_errors_on_missing_dir() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("nonexistent_certs_dir");
    let err = build_tls_config(&path)
        .err()
        .expect("Expected error for missing cert dir");
    assert!(err.to_string().contains("Certificates directory not found"));
}
