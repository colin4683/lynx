use crate::proto::monitor::{ContainerInfo, ContainerMetrics};
use bollard::query_parameters::{
    ListContainersOptions, RestartContainerOptions, StartContainerOptions, StatsOptionsBuilder,
    StopContainerOptions,
};
use bollard::Docker;
use futures_util::TryStreamExt;

pub struct DockerManager {
    docker: Docker,
}

/*
Collect Stats of docker containers:
- CPU Usage:
    ContainerStatsResponse.cpu_stats.cpu_usage.total_usage
    ContainerStatsResponse.cpu_stats.system_cpu_usage?
- Memory usage:
    ContainerStatsResponse.memory_stats.usage
 */
impl DockerManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let docker = Docker::connect_with_local_defaults()?;
        Ok(Self { docker })
    }

    /*
    Available filters:
        ancestor=(<image-name>[:<tag>], <image id>, or <image@digest>)
        before=(<container id> or <container name>)
        expose=(<port>[/<proto>]|<startport-endport>/[<proto>])
        exited=<int> containers with exit code of <int>
        health=(starting|healthy|unhealthy|none)
        id=<ID> a container’s ID
        isolation=(default|process|hyperv) (Windows daemon only)
        is-task=(true|false)
        label=key or label="key=value" of a container label
        name=<name> a container’s name
        network=(<network id> or <network name>)
        publish=(<port>[/<proto>]|<startport-endport>/[<proto>])
        since=(<container id> or <container name>)
        status=(created|restarting|running|removing|paused|exited|dead)
        volume=(<volume name> or <mount point destination>)

        */
    pub async fn list_containers(
        &self,
        options: Option<ListContainersOptions>,
    ) -> Result<Vec<ContainerInfo>, Box<dyn std::error::Error>> {
        let containers = self
            .docker
            .list_containers(Some(options.unwrap_or_default()))
            .await?;
        let containers = containers
            .into_iter()
            .map(|container| ContainerInfo {
                name: container.names.unwrap_or_default().join(","),
                docker_id: container.id.unwrap_or_default(),
                state: container.status.unwrap_or("Unknown".into()),
            })
            .collect();
        Ok(containers)
    }

    pub async fn get_container_stats(
        &self,
        container: &str,
    ) -> Result<Vec<ContainerMetrics>, Box<dyn std::error::Error>> {
        let stats = self
            .docker
            .stats(
                container,
                Some(StatsOptionsBuilder::default().stream(false).build()),
            )
            .try_collect::<Vec<_>>()
            .await?;

        let mapped_stats = stats
            .into_iter()
            .map(|stat| {
                let cpu_stats = stat.cpu_stats.unwrap();
                let memory_stats = stat.memory_stats.unwrap();
                ContainerMetrics {
                    docker_id: container.to_string(),
                    cpu_usage: (cpu_stats.cpu_usage.unwrap().total_usage.unwrap_or(0)
                        / cpu_stats.system_cpu_usage.unwrap_or(0)
                        * 100) as f64,
                    memory_usage: memory_stats.usage.unwrap_or(0) as f64
                        / memory_stats.limit.unwrap_or(0) as f64
                        * 100.0,
                }
            })
            .collect();
        Ok(mapped_stats)
    }

    pub async fn restart_container(
        &self,
        container: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let options: Option<RestartContainerOptions> = None;
        let _ = self.docker.restart_container(container, options).await?;
        Ok(())
    }

    pub async fn start_container(&self, container: &str) -> Result<(), Box<dyn std::error::Error>> {
        let options: Option<StartContainerOptions> = None;
        let _ = self.docker.start_container(container, options).await?;
        Ok(())
    }

    pub async fn stop_container(&self, container: &str) -> Result<(), Box<dyn std::error::Error>> {
        let options: Option<StopContainerOptions> = None;
        let _ = self.docker.stop_container(container, options).await?;
        Ok(())
    }
}
