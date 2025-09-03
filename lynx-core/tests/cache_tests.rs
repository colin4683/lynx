use lynx_core::cache::Cache;
use lynx_core::proto::monitor::SystemService;
use tempfile::tempdir;

#[tokio::test]
async fn cache_snapshot_persists_services_and_logs() {
    let cache = Cache::new(10, 10);
    // Insert services
    for i in 0..3u64 {
        cache.upsert_service(SystemService {
            service_name: format!("svc{i}"),
            description: "test".into(),
            pid: i,
            state: "running".into(),
            cpu: "0%".into(),
            memory: "0".into(),
        });
    }
    // Insert logs
    for i in 0..5 {
        cache.record_log("info", format!("log-{i}")).await;
    }

    assert_eq!(cache.service_count(), 3);
    assert_eq!(cache.log_count().await, 5);

    let dir = tempdir().unwrap();
    let snap_path = dir.path().join("snapshot.bin");
    cache.snapshot_to_file(&snap_path).await.unwrap();
    assert!(snap_path.exists());

    // Load into a new cache instance
    let cache2 = Cache::new(10, 10);
    cache2.load_from_file(&snap_path).await.unwrap();
    assert_eq!(cache2.service_count(), 3, "service count after restore");
    assert_eq!(cache2.log_count().await, 5, "log count after restore");
}

#[tokio::test]
async fn cache_log_trim_works() {
    let cache = Cache::new(5, 5);
    for i in 0..10 {
        cache.record_log("info", format!("l{i}")).await;
    }
    assert_eq!(cache.log_count().await, 5, "should retain only max_logs");
}
