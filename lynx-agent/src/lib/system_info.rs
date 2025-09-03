use crate::lib::cache::FastCache;
use crate::proto::monitor::{
    Component, CpuStats, DiskStats, LoadAverage, MemoryStats, MetricsRequest, NetworkStats,
    SystemInfoRequest, SystemctlRequest,
};
use serde::{Deserialize, Serialize};
#[cfg(target_os = "linux")]
use std::str::FromStr;
use sysinfo::{Components, Networks, ProcessRefreshKind, ProcessesToUpdate, System};
use systemctl::ActiveState;
#[cfg(not(target_os = "windows"))]
use systemstat::Platform;

macro_rules! to_kb {
    ($x:expr) => {
        $x / 1024
    };
}
macro_rules! to_mb {
    ($x:expr) => {
        $x / 1024 / 1024
    };
}
macro_rules! to_gb {
    ($x:expr) => {
        $x / 1024 / 1024 / 1024
    };
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct SystemInfo {
    pub hostname: String,
    pub os: String,
    pub kernal: String,
    pub uptime: u64,
    pub cpu_model: String,
    pub cpu_count: usize,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct BuildSpecs {
    pub cpu_model: String,
    pub cpu_cores: usize,
    pub memory_total: u64,
    pub swap_total: u64,
}

#[derive(Default, Debug, Clone)]
pub struct Metrics {
    pub cpu_stats: CpuStats,
    pub memory_stats: Vec<MemoryStats>,
    pub disk_stats: Vec<DiskStats>,
    pub network_stats: NetworkStats,
    pub components: Vec<Component>,
    pub load_average: LoadAverage,
}

pub async fn collect_system_info(system: &mut System) -> SystemInfoRequest {
    let hostname = sysinfo::System::host_name().unwrap_or(String::from(""));
    let os_info = sysinfo::System::long_os_version().unwrap_or(String::from(""));
    let kernal_version = System::kernel_version().unwrap_or(String::from(""));
    let uptime = System::uptime();
    let boot_time = System::boot_time();
    let users = sysinfo::Users::new_with_refreshed_list().list(); // todo: maybe use?

    let build_specs = BuildSpecs {
        cpu_model: system
            .cpus()
            .first()
            .map(|cpu| cpu.brand().to_string())
            .unwrap_or(String::from("Unknown CPU")),
        cpu_cores: system.cpus().len(),
        memory_total: system.total_memory(),
        swap_total: system.total_swap(),
    };
    SystemInfoRequest {
        hostname,
        os: os_info,
        kernel_version: kernal_version,
        uptime_seconds: uptime,
        cpu_model: build_specs.cpu_model,
        cpu_count: build_specs.cpu_cores as u32,
    }
}

pub async fn collect_systemctl_services(cache: &FastCache) -> SystemctlRequest {
    let systemctl = systemctl::SystemCtl::default();
    let units = systemctl.list_units_full(Some("service"), None, None);
    let mut changed_services = vec![];

    match units {
        Ok(units) => {
            for unit in units {
                // Get current active state and other info
                let active_state = systemctl
                    .get_active_state(&unit.unit_name)
                    .unwrap_or(ActiveState::Unknown);
                let properties = systemctl.create_unit(&unit.unit_name).unwrap_or_default();
                let pid = properties.pid;
                let description = properties.description;
                let enabled = active_state == ActiveState::Active;
                let cpu = properties.cpu;
                let memory = properties.memory;
                // Build SystemService struct
                let service = crate::lib::cache::SystemService {
                    name: unit.unit_name.clone(),
                    status: format!("{:?}", active_state),
                    enabled,
                    description,
                    pid,
                    cpu_usage: cpu,
                    memory_usage: memory,
                };
                // Check cache
                let cached = cache
                    .get_system_service(&unit.unit_name)
                    .await
                    .unwrap_or(None);
                let cached_ref = cached.as_ref();
                if cached.is_none() || cached.as_ref() != Some(&service) {
                    // Update cache if changed or not present
                    let _ = cache
                        .set_system_service(&service, Some(chrono::Duration::minutes(10)))
                        .await;
                    changed_services.push(unit.clone());
                }
            }
        }
        Err(e) => {
            println!("Failed to list systemctl units: {}", e);
        }
    }

    let services = changed_services
        .into_iter()
        .map(|unit| {
            let unit_props = systemctl.create_unit(&unit.unit_name).ok();
            crate::proto::monitor::SystemService {
                service_name: unit.unit_name.clone(),
                description: unit.description,
                state: format!("{:?}", unit.active),
                pid: unit_props.as_ref().and_then(|p| p.pid).unwrap_or(0),
                cpu: unit_props
                    .as_ref()
                    .and_then(|p| p.cpu.clone())
                    .unwrap_or("unknown".to_string()),
                memory: unit_props
                    .as_ref()
                    .and_then(|p| p.memory.clone())
                    .unwrap_or("unknown".to_string()),
            }
        })
        .collect();
    SystemctlRequest { services }
}
fn collect_cpu_stats(system: &System) -> CpuStats {
    let cpu_usage = system
        .cpus()
        .iter()
        .fold(0.0, |acc, cpu| acc + cpu.cpu_usage())
        / system.cpus().len() as f32;
    CpuStats {
        usage_percent: cpu_usage as f64,
    }
}

fn collect_memory_stats(system: &System) -> MemoryStats {
    MemoryStats {
        total_kb: to_kb!(system.total_memory()),
        used_kb: to_kb!(system.used_memory()),
        free_kb: to_kb!(system.free_memory()),
    }
}

#[cfg(target_os = "windows")]
fn collect_component_stats() -> Vec<Component> {
    // Temperature sensors are not supported on Windows by sysinfo
    Vec::new()
}

#[cfg(not(target_os = "windows"))]
fn collect_component_stats() -> Vec<Component> {
    let components = Components::new_with_refreshed_list();
    components
        .iter()
        .map(|c| {
            let temp = c.temperature().unwrap_or(0.0);
            Component {
                label: c.label().to_string(),
                temperature: temp as f32,
            }
        })
        .collect()
}

async fn collect_disk_stats() -> Vec<DiskStats> {
    let sys_disks = sysinfo::Disks::new_with_refreshed_list();
    let disks = sys_disks
        .iter()
        .map(|d| {
            let name = d.name().to_string_lossy().into_owned();
            let mount_point = d.mount_point().to_str().unwrap_or("").to_string();
            let total_space = d.total_space();
            let available_space = d.available_space();
            DiskStats {
                name,
                used_space: to_gb!(total_space - available_space) as i32,
                total_space: to_gb!(total_space) as i32,
                read_bytes: d.usage().total_read_bytes as f64,
                write_bytes: d.usage().total_written_bytes as f64,
                unit: "gb".to_string(),
                mount_point,
            }
        })
        .collect();
    disks
}

#[cfg(target_os = "windows")]
fn collect_load_average(_system: &System) -> LoadAverage {
    // Windows does not support load average, return zeros
    LoadAverage {
        one_minute: 0.0,
        five_minutes: 0.0,
        fifteen_minutes: 0.0,
    }
}

fn collect_load_average(system: &System) -> LoadAverage {
    let load = System::load_average();
    LoadAverage {
        one_minute: load.one,
        five_minutes: load.five,
        fifteen_minutes: load.fifteen,
    }
}

async fn collect_network_stats() -> NetworkStats {
    let get_network_totals = |networks: &sysinfo::Networks| {
        networks
            .values()
            .fold((0, 0), |(mut in_acc, mut out_acc), net| {
                in_acc += net.total_received();
                out_acc += net.total_transmitted();
                (in_acc, out_acc)
            })
    };
    let (net_in, net_out) = get_network_totals(&Networks::new_with_refreshed_list());
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    let (net_in2, net_out2) = get_network_totals(&Networks::new_with_refreshed_list());
    NetworkStats {
        r#in: to_mb!(net_in2 - net_in),
        out: to_mb!(net_out2 - net_out),
    }
}

pub async fn collect_metrics(system: &mut System) -> MetricsRequest {
    system.refresh_all();
    tokio::time::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL).await;
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_cpu(),
    );
    let cpu_stats = collect_cpu_stats(system);
    let memory_stats = collect_memory_stats(system);
    let disk_stats = collect_disk_stats().await;
    let network_stats = collect_network_stats().await;
    let components = collect_component_stats();
    let load_average = collect_load_average(system);
    MetricsRequest {
        cpu_stats: Some(cpu_stats),
        memory_stats: Some(memory_stats),
        disk_stats,
        components,
        network_stats: Some(network_stats),
        load_average: Some(load_average),
    }
}
