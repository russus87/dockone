use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::AtomicU64;
use std::sync::Mutex;

use bollard::Docker;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWrite;

/// A single point in a container's resource-usage history.
#[derive(Debug, Clone, Serialize)]
pub struct Sample {
    pub t: i64,
    pub cpu: f64,
    pub mem: i64,
    pub mem_limit: i64,
}

/// A planned start/stop/restart action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub id: String,
    pub host_id: String,
    pub host_name: String,
    pub container: String,
    pub action: String,
    /// `daily` (fires at `time`) or `interval` (every `every_min` minutes).
    pub kind: String,
    #[serde(default)]
    pub time: String,
    #[serde(default)]
    pub every_min: u64,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub last_run: i64,
}

fn default_true() -> bool {
    true
}

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
    /// Record resource-usage history in the background.
    #[serde(default = "default_true")]
    pub metrics_enabled: bool,
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
            metrics_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedData {
    #[serde(default)]
    pub hosts: Vec<Host>,
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub schedules: Vec<Schedule>,
}

impl Default for PersistedData {
    fn default() -> Self {
        PersistedData {
            hosts: vec![Host::local()],
            settings: Settings::default(),
            schedules: Vec::new(),
        }
    }
}

/// A live SSH port-forward: the `ssh` child process plus the local endpoint
/// (`tcp://127.0.0.1:PORT`) that bollard connects to.
pub struct Tunnel {
    pub child: std::process::Child,
    pub endpoint: String,
}

/// An interactive exec (terminal) session bound to a container.
pub struct ExecSession {
    pub input: Pin<Box<dyn AsyncWrite + Send>>,
    pub exec_id: String,
    pub docker: Docker,
    pub reader: tokio::task::JoinHandle<()>,
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
    /// Live interactive terminal sessions keyed by session id.
    pub execs: tokio::sync::Mutex<HashMap<String, ExecSession>>,
    /// Monotonic counter for terminal session ids.
    pub term_counter: AtomicU64,
    /// Rolling resource-usage history keyed by `host_id/container`.
    pub metrics: Mutex<HashMap<String, VecDeque<Sample>>>,
    /// Live log-follow tasks keyed by session id.
    pub log_tasks: Mutex<HashMap<String, tokio::task::JoinHandle<()>>>,
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
            execs: tokio::sync::Mutex::new(HashMap::new()),
            term_counter: AtomicU64::new(1),
            metrics: Mutex::new(HashMap::new()),
            log_tasks: Mutex::new(HashMap::new()),
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
