use serde::{Deserialize, Serialize};
#[cfg(target_os = "linux")]
use std::collections::HashMap;
use std::str::FromStr;
use log::info;
#[cfg(not(target_os = "windows"))]
use systemstat::{Platform, System as Systemstat};

use crate::proto::monitor::{Component, CpuStats, DiskStats, LoadAverage, MemoryStats, MetricsRequest, NetworkStats, SystemInfoRequest};
use sysinfo::{Components, DiskKind, Disks, Networks, ProcessRefreshKind, ProcessesToUpdate, System, Uid, Users};
use tokio::time::Instant;
/*
- [ ]  System Information
    - [ ]  Hostname
    - [ ]  Operating System
    - [ ]  Kernal Information
    - [ ]  Uptime
    - [ ]  Users
         - [ ] UID
         - [ ] GID
         - [ ] Name
         - [ ] Groups
    - [ ]  Boot time
    - [ ]  System specs (cpu model, motherboard, etc.)
    - [ ]  Load average
- [ ]  Metrics
    - [ ]  CPU Information:
        - [ ]  % usage
    - [ ]  Memory information
        - [ ]  Total
        - [ ]  Used
    - [ ]  Network Information
        - [ ]  Incoming
        - [ ]  Outgoing
    - [ ]  Disk Information
        - [ ]  Disk names
        - [ ]  Space left / used
        - [ ]  I/O
    - [ ]  Components
        - [ ]  Component Name
        - [ ]  Temperature
 */
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

    println!("System Info: {:#?}", build_specs);

    SystemInfoRequest {
        hostname,
        os: os_info,
        kernel_version: kernal_version,
        uptime_seconds: uptime,
        cpu_model: build_specs.cpu_model,
        cpu_count: build_specs.cpu_cores as u32,
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
    
    let cpu_usage = system.cpus().iter().fold(0.0, |acc, cpu| {
        acc + cpu.cpu_usage()
        
    }) / system.cpus().len() as f32;

    let memory_stats = MemoryStats {
        total_kb: system.total_memory() / 1024, // Convert to KB
        used_kb: system.used_memory() / 1024,
        free_kb: system.free_memory() / 1024,
    };
    let sys_disks = sysinfo::Disks::new_with_refreshed_list();
    let mut sys_disks = sys_disks
        .into_iter()
        .filter(|d| {
            ((d.mount_point().to_str().unwrap_or("") == "/")
                || (d.mount_point().to_str().unwrap_or("") == "/mnt"))
        })
        .collect::<Vec<_>>();

    let mut prev_stats: Vec<(String, u64, u64, Instant)> = sys_disks
        .iter()
        .find(|d| d.mount_point().to_str().unwrap_or("") == "/")
        .into_iter()
        .map(|d| (
            d.name().to_string_lossy().into_owned(),
            d.usage().total_read_bytes,
            d.usage().total_written_bytes,
            Instant::now()
        ))
        .collect();

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let sys_disks = sysinfo::Disks::new_with_refreshed_list();

    let disks = sys_disks
        .iter()
        .find(|d| d.mount_point().to_str().unwrap_or("") == "/")
        .into_iter()
        .map(|d| {
            let name = d.name().to_string_lossy().into_owned();
            let mount_point = d.mount_point().to_str().unwrap_or("").to_string();
            let total_space = d.total_space();
            let available_space = d.available_space();
            let current_read = d.usage().total_read_bytes;
            let current_write = d.usage().total_written_bytes;
            let current_time = Instant::now();

            // Find previous stats for this disk
            let binding = (name.clone(), 0, 0, current_time);
            let prev = prev_stats.iter()
                .find(|(n, _, _, _)| n == &name)
                .unwrap_or(&binding);

            // Calculate throughput (bytes per second)
            let time_diff = current_time.duration_since(prev.3).as_secs_f64();
            let read_throughput = if time_diff > 0.0 {
                (current_read - prev.1) as f64 / time_diff
            } else { 0.0 };
            let write_throughput = if time_diff > 0.0 {
                (current_write - prev.2) as f64 / time_diff
            } else { 0.0 };

            // Update previous stats for next iteration
            if let Some(pos) = prev_stats.iter().position(|(n, _, _, _)| n == &name) {
                prev_stats[pos] = (name.clone(), current_read, current_write, current_time);
            }

            DiskStats {
                name: name,
                used_space: ((total_space - available_space) / 1024 / 1024 / 1024) as i32,
                total_space: (total_space / 1024 / 1024 / 1024) as i32,
                read_bytes: read_throughput,
                write_bytes: write_throughput,
                unit: "gb".to_string(),
                mount_point,
            }
        })
        .collect::<Vec<_>>();

    let (net_in, net_out) = sysinfo::Networks::new_with_refreshed_list().values().fold(
        (0, 0),
        |(mut in_acc, mut out_acc), net| {
            in_acc += net.total_received();
            out_acc += net.total_transmitted();
            (in_acc, out_acc)
        },
    );
    
    // sleep for 1 second
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    let (net_in2, net_out2) = sysinfo::Networks::new_with_refreshed_list().values().fold(
        (0, 0),
        |(mut in_acc, mut out_acc), net| {
            in_acc += net.total_received();
            out_acc += net.total_transmitted();
            (in_acc, out_acc)
        },
    );
    
    let net_in = (net_in2 - net_in) / 1024; // Convert to KB
    let net_out = (net_out2 - net_out) / 1024; // Convert to KB

    let components = Components::new_with_refreshed_list();
    let components_dump = components
        .iter()
        .map(|c| {
            let temp = c.temperature().unwrap_or(0.0);
            Component {
                label: c.label().to_string(),
                temperature: temp,
            }
        })
        .collect::<Vec<Component>>();

    let load_average = LoadAverage {
        one_minute: System::load_average().one,
        five_minutes: System::load_average().five,
        fifteen_minutes: System::load_average().fifteen,
    };

    MetricsRequest {
        cpu_stats: Option::from(CpuStats {
            usage_percent: cpu_usage as f64,
        }),
        memory_stats: Option::from(memory_stats),
        disk_stats: disks,
        network_stats: Option::from(NetworkStats {
            r#in: net_in,
            r#out: net_out,
        }),
        components: components_dump,
        load_average: Option::from(load_average),
    }
}
