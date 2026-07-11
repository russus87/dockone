//! Thin async wrapper over `bollard`. Every function takes a resolved
//! `Docker` handle and returns plain serde DTOs so the frontend never sees
//! bollard's model types.

use bollard::container::{
    ListContainersOptions, LogsOptions, RestartContainerOptions, StartContainerOptions,
    StopContainerOptions,
};
use bollard::image::ListImagesOptions;
use bollard::Docker;
use futures_util::StreamExt;
use serde::Serialize;

use crate::state::Host;

/// Open (or reuse the cached) connection for a host definition.
pub fn connect(host: &Host) -> Result<Docker, String> {
    let ep = host.endpoint.trim();
    let docker = if ep.is_empty() || ep == "local" {
        Docker::connect_with_local_defaults()
    } else if let Some(rest) = ep.strip_prefix("unix://") {
        Docker::connect_with_unix(rest, 120, bollard::API_DEFAULT_VERSION)
    } else {
        // tcp:// and http:// both speak the plain HTTP Docker API.
        let addr = ep.replace("tcp://", "http://");
        Docker::connect_with_http(&addr, 120, bollard::API_DEFAULT_VERSION)
    };
    docker.map_err(|e| format!("connessione a «{}» fallita: {e}", host.name))
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

            ContainerDto {
                id: c.id.unwrap_or_default(),
                name: c
                    .names
                    .and_then(|n| n.into_iter().next())
                    .map(|n| n.trim_start_matches('/').to_string())
                    .unwrap_or_default(),
                image: c.image.unwrap_or_default(),
                state: c.state.unwrap_or_default(),
                status: c.status.unwrap_or_default(),
                ports,
                created: c.created.unwrap_or(0),
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

fn err(e: bollard::errors::Error) -> String {
    e.to_string()
}
