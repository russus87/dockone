use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use bollard::Docker;
use serde::{Deserialize, Serialize};

/// A Docker endpoint the user manages — the local socket or a remote host.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Host {
    pub id: String,
    pub name: String,
    /// `local` for the platform default socket, or a `tcp://host:port` /
    /// `unix:///path` / `npipe://...` connection string.
    pub endpoint: String,
    #[serde(default)]
    pub favorite: bool,
}

impl Host {
    fn local() -> Self {
        Host {
            id: "local".into(),
            name: "Local".into(),
            endpoint: "local".into(),
            favorite: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_theme")]
    pub theme: String,
    /// When true, mutating actions (start/stop/prune/…) are rejected.
    #[serde(default)]
    pub read_only: bool,
    /// Container ids/names marked as favorites (any host).
    #[serde(default)]
    pub favorite_containers: Vec<String>,
    /// Background watcher that notifies on container crashes / unhealthy state.
    #[serde(default)]
    pub alerts_enabled: bool,
    #[serde(default = "default_poll")]
    pub alert_poll_secs: u64,
}

fn default_theme() -> String {
    "light".into()
}

fn default_poll() -> u64 {
    30
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: default_theme(),
            read_only: false,
            favorite_containers: Vec::new(),
            alerts_enabled: false,
            alert_poll_secs: default_poll(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedData {
    #[serde(default)]
    pub hosts: Vec<Host>,
    #[serde(default)]
    pub settings: Settings,
}

impl Default for PersistedData {
    fn default() -> Self {
        PersistedData {
            hosts: vec![Host::local()],
            settings: Settings::default(),
        }
    }
}

/// A live SSH port-forward: the `ssh` child process plus the local endpoint
/// (`tcp://127.0.0.1:PORT`) that bollard connects to.
pub struct Tunnel {
    pub child: std::process::Child,
    pub endpoint: String,
}

/// Shared application state: persisted config + a cache of live Docker handles.
pub struct AppState {
    pub data: Mutex<PersistedData>,
    /// Cached connections keyed by host id (bollard `Docker` is cheap to clone).
    pub conns: Mutex<HashMap<String, Docker>>,
    /// Last seen state per `host_id/name`, used by the alerts watcher.
    pub last_states: Mutex<HashMap<String, String>>,
    /// Active SSH tunnels keyed by host id.
    pub tunnels: Mutex<HashMap<String, Tunnel>>,
}

impl AppState {
    pub fn load() -> Self {
        let data = std::fs::read_to_string(config_path())
            .ok()
            .and_then(|s| serde_json::from_str::<PersistedData>(&s).ok())
            .map(|mut d| {
                if d.hosts.is_empty() {
                    d.hosts.push(Host::local());
                }
                d
            })
            .unwrap_or_default();

        AppState {
            data: Mutex::new(data),
            conns: Mutex::new(HashMap::new()),
            last_states: Mutex::new(HashMap::new()),
            tunnels: Mutex::new(HashMap::new()),
        }
    }

    pub fn save(&self) {
        if let Ok(d) = self.data.lock() {
            let path = config_path();
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string_pretty(&*d) {
                let _ = std::fs::write(path, json);
            }
        }
    }
}

fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("dockone")
}

fn config_path() -> PathBuf {
    config_dir().join("config.json")
}
