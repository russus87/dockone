//! Thin async wrapper over `bollard`. Every function takes a resolved
//! `Docker` handle and returns plain serde DTOs so the frontend never sees
//! bollard's model types.

use bollard::container::{
    Config, CreateContainerOptions, ListContainersOptions, LogsOptions, RestartContainerOptions,
    StartContainerOptions, StopContainerOptions,
};
use bollard::image::ListImagesOptions;
use bollard::models::{HostConfig, PortBinding, RestartPolicy, RestartPolicyNameEnum};
use bollard::Docker;
use futures_util::future::join_all;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::state::Host;

/// Open (or reuse the cached) connection for a host definition.
pub fn connect(host: &Host) -> Result<Docker, String> {
    let ep = host.endpoint.trim();
    let docker = if ep.is_empty() || ep == "local" {
        Docker::connect_with_local_defaults()
    } else if let Some(path) = ep.strip_prefix("unix://") {
        connect_unix(path)
    } else {
        // tcp:// and http:// both speak the plain HTTP Docker API.
        let addr = ep.replace("tcp://", "http://");
        Docker::connect_with_http(&addr, 120, bollard::API_DEFAULT_VERSION)
    };
    docker.map_err(|e| format!("connessione a «{}» fallita: {e}", host.name))
}

#[cfg(unix)]
fn connect_unix(path: &str) -> Result<Docker, bollard::errors::Error> {
    Docker::connect_with_unix(path, 120, bollard::API_DEFAULT_VERSION)
}

#[cfg(not(unix))]
fn connect_unix(_path: &str) -> Result<Docker, bollard::errors::Error> {
    // Unix domain sockets aren't available here (e.g. Windows) — fall back to
    // the platform default endpoint (named pipe on Windows).
    Docker::connect_with_local_defaults()
}

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct HostSummary {
    pub id: String,
    pub name: String,
    pub online: bool,
    pub error: Option<String>,
    pub containers: i64,
    pub running: i64,
    pub stopped: i64,
    pub paused: i64,
    pub images: i64,
    pub cpus: i64,
    pub mem_total: i64,
    pub docker_version: Option<String>,
    pub os: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContainerDto {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: String,
    pub status: String,
    pub ports: Vec<String>,
    pub created: i64,
    /// healthy | unhealthy | starting | none (parsed from the status line)
    pub health: String,
    /// docker-compose project name, if the container belongs to a stack
    pub compose: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImageDto {
    pub id: String,
    pub tags: Vec<String>,
    pub size: i64,
    pub created: i64,
    pub containers: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct VolumeDto {
    pub name: String,
    pub driver: String,
    pub mountpoint: String,
    pub created: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkDto {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub scope: String,
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

pub async fn host_summary(host: &Host, docker: &Docker) -> HostSummary {
    let mut s = HostSummary {
        id: host.id.clone(),
        name: host.name.clone(),
        online: false,
        error: None,
        containers: 0,
        running: 0,
        stopped: 0,
        paused: 0,
        images: 0,
        cpus: 0,
        mem_total: 0,
        docker_version: None,
        os: None,
    };

    match docker.info().await {
        Ok(info) => {
            s.online = true;
            s.containers = info.containers.unwrap_or(0);
            s.running = info.containers_running.unwrap_or(0);
            s.stopped = info.containers_stopped.unwrap_or(0);
            s.paused = info.containers_paused.unwrap_or(0);
            s.images = info.images.unwrap_or(0);
            s.cpus = info.ncpu.unwrap_or(0);
            s.mem_total = info.mem_total.unwrap_or(0);
            s.os = info.operating_system;
            s.docker_version = info.server_version;
        }
        Err(e) => {
            s.error = Some(e.to_string());
        }
    }
    s
}

pub async fn list_containers(docker: &Docker) -> Result<Vec<ContainerDto>, String> {
    let opts = ListContainersOptions::<String> {
        all: true,
        ..Default::default()
    };
    let list = docker.list_containers(Some(opts)).await.map_err(err)?;

    let out = list
        .into_iter()
        .map(|c| {
            let ports = c
                .ports
                .unwrap_or_default()
                .into_iter()
                .filter_map(|p| {
                    let proto = p
                        .typ
                        .map(|t| format!("{t:?}").to_lowercase())
                        .unwrap_or_else(|| "tcp".into());
                    match p.public_port {
                        Some(pub_p) => Some(format!("{pub_p}->{}/{proto}", p.private_port)),
                        None => Some(format!("{}/{proto}", p.private_port)),
                    }
                })
                .collect::<Vec<_>>();

            let status = c.status.unwrap_or_default();
            let compose = c
                .labels
                .as_ref()
                .and_then(|m| m.get("com.docker.compose.project").cloned());

            ContainerDto {
                id: c.id.unwrap_or_default(),
                name: c
                    .names
                    .and_then(|n| n.into_iter().next())
                    .map(|n| n.trim_start_matches('/').to_string())
                    .unwrap_or_default(),
                image: c.image.unwrap_or_default(),
                state: c.state.unwrap_or_default(),
                health: parse_health(&status),
                status,
                ports,
                created: c.created.unwrap_or(0),
                compose,
            }
        })
        .collect();
    Ok(out)
}

pub async fn list_images(docker: &Docker) -> Result<Vec<ImageDto>, String> {
    let opts = ListImagesOptions::<String> {
        all: false,
        ..Default::default()
    };
    let list = docker.list_images(Some(opts)).await.map_err(err)?;
    Ok(list
        .into_iter()
        .map(|i| ImageDto {
            id: i.id.trim_start_matches("sha256:").chars().take(12).collect(),
            tags: i
                .repo_tags
                .into_iter()
                .filter(|t| t != "<none>:<none>")
                .collect(),
            size: i.size,
            created: i.created,
            containers: i.containers,
        })
        .collect())
}

pub async fn list_volumes(docker: &Docker) -> Result<Vec<VolumeDto>, String> {
    let resp = docker
        .list_volumes(None::<bollard::volume::ListVolumesOptions<String>>)
        .await
        .map_err(err)?;
    Ok(resp
        .volumes
        .unwrap_or_default()
        .into_iter()
        .map(|v| VolumeDto {
            name: v.name,
            driver: v.driver,
            mountpoint: v.mountpoint,
            created: v.created_at,
        })
        .collect())
}

pub async fn list_networks(docker: &Docker) -> Result<Vec<NetworkDto>, String> {
    let list = docker
        .list_networks(None::<bollard::network::ListNetworksOptions<String>>)
        .await
        .map_err(err)?;
    Ok(list
        .into_iter()
        .map(|n| NetworkDto {
            id: n.id.unwrap_or_default().chars().take(12).collect(),
            name: n.name.unwrap_or_default(),
            driver: n.driver.unwrap_or_default(),
            scope: n.scope.unwrap_or_default(),
        })
        .collect())
}

pub async fn container_action(docker: &Docker, id: &str, action: &str) -> Result<(), String> {
    match action {
        "start" => docker
            .start_container(id, None::<StartContainerOptions<String>>)
            .await
            .map_err(err),
        "stop" => docker
            .stop_container(id, None::<StopContainerOptions>)
            .await
            .map_err(err),
        "restart" => docker
            .restart_container(id, None::<RestartContainerOptions>)
            .await
            .map_err(err),
        "pause" => docker.pause_container(id).await.map_err(err),
        "unpause" => docker.unpause_container(id).await.map_err(err),
        "kill" => docker
            .kill_container(id, None::<bollard::container::KillContainerOptions<String>>)
            .await
            .map_err(err),
        other => Err(format!("azione sconosciuta: {other}")),
    }
}

pub async fn container_logs(docker: &Docker, id: &str, tail: &str) -> Result<String, String> {
    let opts = LogsOptions::<String> {
        stdout: true,
        stderr: true,
        timestamps: false,
        tail: tail.to_string(),
        ..Default::default()
    };
    let mut stream = docker.logs(id, Some(opts));
    let mut out = String::new();
    while let Some(item) = stream.next().await {
        match item {
            Ok(chunk) => out.push_str(&String::from_utf8_lossy(&chunk.into_bytes())),
            Err(e) => return Err(err(e)),
        }
    }
    Ok(out)
}

pub async fn inspect_container(docker: &Docker, id: &str) -> Result<serde_json::Value, String> {
    let details = docker.inspect_container(id, None).await.map_err(err)?;
    serde_json::to_value(details).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Live stats
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct StatDto {
    pub id: String,
    pub name: String,
    pub cpu_percent: f64,
    pub mem_used: i64,
    pub mem_limit: i64,
    pub mem_percent: f64,
    pub net_rx: i64,
    pub net_tx: i64,
    pub blk_read: i64,
    pub blk_write: i64,
}

pub async fn container_stats(docker: &Docker) -> Result<Vec<StatDto>, String> {
    let opts = ListContainersOptions::<String> {
        all: false,
        ..Default::default()
    };
    let list = docker.list_containers(Some(opts)).await.map_err(err)?;
    let targets: Vec<(String, String)> = list
        .into_iter()
        .filter_map(|c| {
            let id = c.id?;
            let name = c
                .names
                .and_then(|n| n.into_iter().next())
                .map(|n| n.trim_start_matches('/').to_string())
                .unwrap_or_default();
            Some((id, name))
        })
        .collect();

    let futs = targets.into_iter().map(|(id, name)| {
        let docker = docker.clone();
        async move { one_stat(&docker, &id, &name).await }
    });
    Ok(join_all(futs).await.into_iter().flatten().collect())
}

async fn one_stat(docker: &Docker, id: &str, name: &str) -> Option<StatDto> {
    use bollard::container::StatsOptions;
    let mut stream = docker.stats(
        id,
        Some(StatsOptions {
            stream: true,
            one_shot: false,
        }),
    );
    // The first frame has an empty precpu snapshot; read a second frame so the
    // CPU delta is meaningful.
    let mut last = None;
    for _ in 0..2 {
        match stream.next().await {
            Some(Ok(s)) => last = Some(s),
            _ => break,
        }
    }
    let s = last?;

    let cpu_delta =
        s.cpu_stats.cpu_usage.total_usage as f64 - s.precpu_stats.cpu_usage.total_usage as f64;
    let sys_delta = s.cpu_stats.system_cpu_usage.unwrap_or(0) as f64
        - s.precpu_stats.system_cpu_usage.unwrap_or(0) as f64;
    let cpus = s
        .cpu_stats
        .online_cpus
        .or_else(|| {
            s.cpu_stats
                .cpu_usage
                .percpu_usage
                .as_ref()
                .map(|v| v.len() as u64)
        })
        .unwrap_or(1) as f64;
    let cpu_percent = if sys_delta > 0.0 && cpu_delta > 0.0 {
        (cpu_delta / sys_delta) * cpus * 100.0
    } else {
        0.0
    };

    let mem_used = s.memory_stats.usage.unwrap_or(0) as i64;
    let mem_limit = s.memory_stats.limit.unwrap_or(0) as i64;
    let mem_percent = if mem_limit > 0 {
        mem_used as f64 / mem_limit as f64 * 100.0
    } else {
        0.0
    };

    let (mut rx, mut tx) = (0i64, 0i64);
    if let Some(nets) = &s.networks {
        for n in nets.values() {
            rx += n.rx_bytes as i64;
            tx += n.tx_bytes as i64;
        }
    }
    let (mut br, mut bw) = (0i64, 0i64);
    if let Some(entries) = &s.blkio_stats.io_service_bytes_recursive {
        for e in entries {
            match e.op.to_lowercase().as_str() {
                "read" => br += e.value as i64,
                "write" => bw += e.value as i64,
                _ => {}
            }
        }
    }

    Some(StatDto {
        id: id.chars().take(12).collect(),
        name: name.to_string(),
        cpu_percent,
        mem_used,
        mem_limit,
        mem_percent,
        net_rx: rx,
        net_tx: tx,
        blk_read: br,
        blk_write: bw,
    })
}

// ---------------------------------------------------------------------------
// Events timeline
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct EventDto {
    pub time: i64,
    pub kind: String,
    pub action: String,
    pub actor: String,
}

pub async fn recent_events(docker: &Docker, since: i64, until: i64) -> Result<Vec<EventDto>, String> {
    use bollard::system::EventsOptions;
    let opts = EventsOptions::<String> {
        since: Some(since.to_string()),
        until: Some(until.to_string()),
        filters: Default::default(),
    };
    let mut stream = docker.events(Some(opts));
    let mut out = Vec::new();
    while let Some(item) = stream.next().await {
        match item {
            Ok(ev) => {
                let actor = ev
                    .actor
                    .as_ref()
                    .and_then(|a| a.attributes.as_ref())
                    .and_then(|m| m.get("name").cloned())
                    .or_else(|| {
                        ev.actor
                            .as_ref()
                            .and_then(|a| a.id.clone())
                            .map(|i| i.chars().take(12).collect())
                    })
                    .unwrap_or_default();
                out.push(EventDto {
                    time: ev.time.unwrap_or(0),
                    kind: ev
                        .typ
                        .map(|t| format!("{t:?}").to_lowercase())
                        .unwrap_or_default(),
                    action: ev.action.unwrap_or_default(),
                    actor,
                });
            }
            Err(_) => break,
        }
    }
    out.reverse(); // newest first
    Ok(out)
}

// ---------------------------------------------------------------------------
// Disk usage + prune
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct DfDto {
    pub images_size: i64,
    pub images_count: i64,
    pub volumes_size: i64,
    pub volumes_count: i64,
    pub containers_count: i64,
}

pub async fn system_df(docker: &Docker) -> Result<DfDto, String> {
    let d = docker.df().await.map_err(err)?;
    let images = d.images.unwrap_or_default();
    let volumes = d.volumes.unwrap_or_default();
    let volumes_size: i64 = volumes
        .iter()
        .filter_map(|v| v.usage_data.as_ref().map(|u| u.size))
        .sum();
    let containers = d.containers.unwrap_or_default();
    Ok(DfDto {
        images_size: d.layers_size.unwrap_or(0),
        images_count: images.len() as i64,
        volumes_size,
        volumes_count: volumes.len() as i64,
        containers_count: containers.len() as i64,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct PruneResult {
    pub reclaimed: i64,
    pub detail: String,
}

pub async fn prune(docker: &Docker, kind: &str) -> Result<PruneResult, String> {
    match kind {
        "containers" => {
            let r = docker
                .prune_containers(None::<bollard::container::PruneContainersOptions<String>>)
                .await
                .map_err(err)?;
            Ok(PruneResult {
                reclaimed: r.space_reclaimed.unwrap_or(0),
                detail: format!("{} container rimossi", r.containers_deleted.unwrap_or_default().len()),
            })
        }
        "images" => {
            let r = docker
                .prune_images(None::<bollard::image::PruneImagesOptions<String>>)
                .await
                .map_err(err)?;
            Ok(PruneResult {
                reclaimed: r.space_reclaimed.unwrap_or(0),
                detail: format!("{} immagini rimosse", r.images_deleted.unwrap_or_default().len()),
            })
        }
        "volumes" => {
            let r = docker
                .prune_volumes(None::<bollard::volume::PruneVolumesOptions<String>>)
                .await
                .map_err(err)?;
            Ok(PruneResult {
                reclaimed: r.space_reclaimed.unwrap_or(0),
                detail: format!("{} volumi rimossi", r.volumes_deleted.unwrap_or_default().len()),
            })
        }
        "networks" => {
            let r = docker
                .prune_networks(None::<bollard::network::PruneNetworksOptions<String>>)
                .await
                .map_err(err)?;
            Ok(PruneResult {
                reclaimed: 0,
                detail: format!("{} reti rimosse", r.networks_deleted.unwrap_or_default().len()),
            })
        }
        other => Err(format!("tipo prune sconosciuto: {other}")),
    }
}

// ---------------------------------------------------------------------------
// Image pull + resource removal + exec
// ---------------------------------------------------------------------------

pub async fn pull_image(docker: &Docker, image: &str) -> Result<(), String> {
    use bollard::image::CreateImageOptions;
    let opts = CreateImageOptions {
        from_image: image.to_string(),
        ..Default::default()
    };
    let mut stream = docker.create_image(Some(opts), None, None);
    while let Some(item) = stream.next().await {
        item.map_err(err)?;
    }
    Ok(())
}

pub async fn remove_image(docker: &Docker, id: &str) -> Result<(), String> {
    use bollard::image::RemoveImageOptions;
    docker
        .remove_image(
            id,
            Some(RemoveImageOptions {
                force: true,
                ..Default::default()
            }),
            None,
        )
        .await
        .map_err(err)?;
    Ok(())
}

pub async fn remove_container(docker: &Docker, id: &str) -> Result<(), String> {
    use bollard::container::RemoveContainerOptions;
    docker
        .remove_container(
            id,
            Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            }),
        )
        .await
        .map_err(err)
}

pub async fn remove_volume(docker: &Docker, name: &str) -> Result<(), String> {
    use bollard::volume::RemoveVolumeOptions;
    docker
        .remove_volume(name, Some(RemoveVolumeOptions { force: true }))
        .await
        .map_err(err)
}

pub async fn remove_network(docker: &Docker, id: &str) -> Result<(), String> {
    docker.remove_network(id).await.map_err(err)
}

pub async fn exec_run(docker: &Docker, id: &str, cmd: &str) -> Result<String, String> {
    use bollard::exec::{CreateExecOptions, StartExecOptions, StartExecResults};
    let exec = docker
        .create_exec(
            id,
            CreateExecOptions {
                cmd: Some(vec!["/bin/sh", "-c", cmd]),
                attach_stdout: Some(true),
                attach_stderr: Some(true),
                ..Default::default()
            },
        )
        .await
        .map_err(err)?;

    let mut out = String::new();
    if let StartExecResults::Attached { mut output, .. } = docker
        .start_exec(&exec.id, None::<StartExecOptions>)
        .await
        .map_err(err)?
    {
        while let Some(item) = output.next().await {
            match item {
                Ok(msg) => out.push_str(&String::from_utf8_lossy(&msg.into_bytes())),
                Err(e) => return Err(err(e)),
            }
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Deploy / clone
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PortMap {
    pub host: String,
    pub container: String,
    pub proto: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolMap {
    pub host: String,
    pub container: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeploySpec {
    pub name: String,
    pub image: String,
    pub cmd: Vec<String>,
    pub env: Vec<String>,
    pub ports: Vec<PortMap>,
    pub volumes: Vec<VolMap>,
    /// no | always | unless-stopped | on-failure
    pub restart: String,
}

pub async fn create_container(
    docker: &Docker,
    spec: DeploySpec,
    autostart: bool,
) -> Result<String, String> {
    if spec.image.trim().is_empty() {
        return Err("L'immagine è obbligatoria".into());
    }

    let mut exposed: HashMap<String, HashMap<(), ()>> = HashMap::new();
    let mut bindings: HashMap<String, Option<Vec<PortBinding>>> = HashMap::new();
    for p in &spec.ports {
        if p.container.trim().is_empty() {
            continue;
        }
        let proto = if p.proto.is_empty() { "tcp" } else { &p.proto };
        let key = format!("{}/{}", p.container.trim(), proto);
        exposed.insert(key.clone(), HashMap::new());
        if !p.host.trim().is_empty() {
            bindings.insert(
                key,
                Some(vec![PortBinding {
                    host_ip: None,
                    host_port: Some(p.host.trim().to_string()),
                }]),
            );
        }
    }

    let binds: Vec<String> = spec
        .volumes
        .iter()
        .filter(|v| !v.host.trim().is_empty() && !v.container.trim().is_empty())
        .map(|v| format!("{}:{}", v.host.trim(), v.container.trim()))
        .collect();

    let restart_name = match spec.restart.as_str() {
        "always" => RestartPolicyNameEnum::ALWAYS,
        "unless-stopped" => RestartPolicyNameEnum::UNLESS_STOPPED,
        "on-failure" => RestartPolicyNameEnum::ON_FAILURE,
        _ => RestartPolicyNameEnum::NO,
    };

    let host_config = HostConfig {
        port_bindings: if bindings.is_empty() { None } else { Some(bindings) },
        binds: if binds.is_empty() { None } else { Some(binds) },
        restart_policy: Some(RestartPolicy {
            name: Some(restart_name),
            maximum_retry_count: None,
        }),
        ..Default::default()
    };

    let config: Config<String> = Config {
        image: Some(spec.image.trim().to_string()),
        cmd: if spec.cmd.is_empty() { None } else { Some(spec.cmd.clone()) },
        env: if spec.env.is_empty() { None } else { Some(spec.env.clone()) },
        exposed_ports: if exposed.is_empty() { None } else { Some(exposed) },
        host_config: Some(host_config),
        ..Default::default()
    };

    let opts = spec.name.trim().is_empty().then(|| None).unwrap_or_else(|| {
        Some(CreateContainerOptions {
            name: spec.name.trim().to_string(),
            platform: None,
        })
    });

    let res = docker.create_container(opts, config).await.map_err(err)?;
    if autostart {
        docker
            .start_container(&res.id, None::<StartContainerOptions<String>>)
            .await
            .map_err(err)?;
    }
    Ok(res.id)
}

/// Read an existing container's configuration back into a `DeploySpec` (clone).
pub async fn container_config(docker: &Docker, id: &str) -> Result<DeploySpec, String> {
    let d = docker.inspect_container(id, None).await.map_err(err)?;
    let cfg = d.config.unwrap_or_default();
    let hc = d.host_config.unwrap_or_default();
    let name = d
        .name
        .unwrap_or_default()
        .trim_start_matches('/')
        .to_string();

    let mut ports = Vec::new();
    if let Some(pb) = hc.port_bindings {
        for (k, v) in pb {
            let (cport, proto) = k.split_once('/').unwrap_or((k.as_str(), "tcp"));
            let host = v
                .and_then(|vec| vec.into_iter().next())
                .and_then(|b| b.host_port)
                .unwrap_or_default();
            ports.push(PortMap {
                host,
                container: cport.to_string(),
                proto: proto.to_string(),
            });
        }
    }

    let volumes = hc
        .binds
        .unwrap_or_default()
        .into_iter()
        .filter_map(|b| {
            let mut it = b.splitn(2, ':');
            Some(VolMap {
                host: it.next()?.to_string(),
                container: it.next()?.to_string(),
            })
        })
        .collect();

    let restart = hc
        .restart_policy
        .and_then(|r| r.name)
        .map(|n| match n {
            RestartPolicyNameEnum::ALWAYS => "always",
            RestartPolicyNameEnum::UNLESS_STOPPED => "unless-stopped",
            RestartPolicyNameEnum::ON_FAILURE => "on-failure",
            _ => "no",
        })
        .unwrap_or("no")
        .to_string();

    Ok(DeploySpec {
        name: format!("{name}-clone"),
        image: cfg.image.unwrap_or_default(),
        cmd: cfg.cmd.unwrap_or_default(),
        env: cfg.env.unwrap_or_default(),
        ports,
        volumes,
        restart,
    })
}

fn parse_health(status: &str) -> String {
    let s = status.to_lowercase();
    if s.contains("unhealthy") {
        "unhealthy".into()
    } else if s.contains("healthy") {
        "healthy".into()
    } else if s.contains("health: starting") || s.contains("starting") {
        "starting".into()
    } else {
        "none".into()
    }
}

fn err(e: bollard::errors::Error) -> String {
    e.to_string()
}
